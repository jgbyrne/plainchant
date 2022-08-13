use crate::actions;
use crate::db;
use crate::fr;
use crate::pages;
use crate::template::{Data, Template};
use crate::Config;

use axum::handler::Handler;
use axum::http::{StatusCode, Uri};
use axum::response::{ErrorResponse, Html, IntoResponse, IntoResponseParts};
use axum::{body, extract, response, routing, Router};

use tokio;
use tokio_util::io::ReaderStream;

use bytes::{BufMut, Bytes, BytesMut};
use mime_guess;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::path;
use std::sync::{Arc, RwLock};

macro_rules! unwrap_or_return {
    ( $test:expr, $ret:expr ) => {
        match $test {
            Ok(val) => val,
            Err(_) => {
                return $ret;
            },
        }
    };
}

// This value is equivalent to 64 MiB in bytes;
const FORM_MAX_LENGTH: u64 = 67_108_864;
// This values is equivalent to 2 MiB in bytes;
const FILE_MAX_SIZE: usize = 2_097_152;

// More complex templates are handled by Pages, but these
// simple message pages we can handle directly
struct StaticPages {
    error_tmpl:   Template,
    message_tmpl: Template,
}

// Utility functions to generate static pages

fn error_page(sp: &StaticPages, message: &str) -> Html<String> {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    Html::from(sp.error_tmpl.render(&Data::values(vals)))
}

fn message_page(sp: &StaticPages, message: &str) -> Html<String> {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    Html::from(sp.message_tmpl.render(&Data::values(vals)))
}

fn internal_error(sp: &StaticPages, message: &str) -> (StatusCode, Html<String>) {
    (StatusCode::INTERNAL_SERVER_ERROR, error_page(sp, message))
}

fn bad_request(sp: &StaticPages, message: &str) -> (StatusCode, Html<String>) {
    (StatusCode::BAD_REQUEST, message_page(sp, message))
}

fn not_found(sp: &StaticPages, message: &str) -> (StatusCode, Html<String>) {
    (StatusCode::NOT_FOUND, message_page(sp, message))
}

fn ok_page(page: &pages::Page) -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html(page.page_text.to_string()))
}

fn render_page<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    db: Arc<DB>,
    page_ref: &pages::PageRef,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    let page = {
        let pg = unwrap_or_return!(pages.read(), {
            internal_error(&sp, "Could not gain read access to Pages")
        });

        match pg.render(db.as_ref(), &page_ref) {
            Ok(page) => page,
            Err(_) => {
                return internal_error(&sp, "Failed to render page");
            },
        }
    };

    // Only grab the write-lock for inserting into the page map

    let mut pg = unwrap_or_return!(pages.write(), {
        internal_error(&sp, "Could not gain write access to Pages")
    });

    let pages = pg.deref_mut();
    let page = pages.update(&page_ref, page);
    ok_page(page)
}

// static_dir: Handler to serve static resources

async fn static_dir(
    config: Arc<Config>,
    extract::Path(path): extract::Path<path::PathBuf>,
) -> impl IntoResponse {
    let path = path.strip_prefix("/").unwrap();
    let full_path = config.static_dir.join(&path);

    match tokio::fs::File::open(&full_path).await {
        Ok(file) => {
            let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
            let headers = response::AppendHeaders([("Content-Type", mime.to_string())]);

            let stream = ReaderStream::new(file);
            let body = body::StreamBody::new(stream);

            Ok((headers, body))
        },
        Err(_err) => Err((
            StatusCode::NOT_FOUND,
            format!("Could not retrieve static file: {}", path.to_string_lossy()),
        )),
    }
}

// thread: Handler to serve thread pages

async fn thread<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    db: Arc<DB>,
    extract::Path((board, post_num)): extract::Path<(String, u64)>,
) -> Result<(StatusCode, Html<String>), ErrorResponse>
where
    DB: 'static + db::Database + Sync + Send,
{
    let page_ref = {
        let pg = unwrap_or_return!(pages.read(), {
            Ok(internal_error(&sp, "Could not gain read access to Pages"))
        });

        let board_id = unwrap_or_return!(pg.board_url_to_id(&board), {
            Ok(not_found(&sp, "No such board"))
        });

        let page_ref = pages::PageRef::Thread(board_id, post_num);

        match pg.get_page(db.as_ref(), &page_ref) {
            Ok(None) => page_ref,
            Ok(Some(page)) => {
                return Ok(ok_page(page));
            },
            Err(_) => {
                // The board exists but the original post does not
                // Let's try and fetch it as a reply
                let reply = unwrap_or_return!(db.get_reply(board_id, post_num), {
                    Ok(not_found(&sp, "No such thread"))
                });
                let uri = format!("/{}/thread/{}#{}", &board, reply.orig_num, post_num);
                return Err(response::Redirect::permanent(&uri).into());
            },
        }
    };

    Ok(render_page(sp, pages, db, &page_ref))
}

// homepage: Handler to serve homepage

async fn homepage<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    db: Arc<DB>,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    let page_ref = pages::PageRef::Homepage;

    {
        let pg = unwrap_or_return!(pages.read(), {
            internal_error(&sp, "Could not gain read access to Pages")
        });

        if let Some(page) = pg
            .get_page(db.as_ref(), &page_ref)
            .expect("Could not access homepage")
        {
            return ok_page(page);
        }
    }

    render_page(sp, pages, db, &page_ref)
}

// catalog: Handler to serve catalog pages

async fn catalog<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    db: Arc<DB>,
    extract::Path(board): extract::Path<String>,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    let page_ref = {
        let pg = unwrap_or_return!(pages.read(), {
            internal_error(&sp, "Could not gain read access to Pages")
        });

        let board_id = unwrap_or_return!(pg.board_url_to_id(&board), {
            not_found(&sp, "No such board")
        });

        let page_ref = pages::PageRef::Catalog(board_id);
        match pg
            .get_page(db.as_ref(), &page_ref)
            .expect("Could not access catalog for extant board")
        {
            Some(page) => {
                return ok_page(page);
            },
            None => page_ref,
        }
    };

    render_page(sp, pages, db, &page_ref)
}

// create: Handler to serve original post creation page

async fn create<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    db: Arc<DB>,
    extract::Path(board): extract::Path<String>,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    let page_ref = {
        let pg = unwrap_or_return!(pages.read(), {
            internal_error(&sp, "Could not gain read access to Pages")
        });

        let board_id = unwrap_or_return!(pg.board_url_to_id(&board), {
            not_found(&sp, "No such board")
        });

        let page_ref = pages::PageRef::Create(board_id);
        match pg
            .get_page(db.as_ref(), &page_ref)
            .expect("Could not access thread creation page for extant board")
        {
            Some(page) => {
                return ok_page(page);
            },
            None => page_ref,
        }
    };

    render_page(sp, pages, db, &page_ref)
}

// Parse a multipart text field

async fn multipart_text_field<'f>(
    sp: &StaticPages,
    field: extract::multipart::Field<'f>,
    max_length: usize,
) -> Result<Option<String>, (StatusCode, Html<String>)> {
    let txt = field
        .text()
        .await
        .map_err(|_err| bad_request(sp, "Could not parse text field"))?;

    if txt.len() > max_length {
        Err(bad_request(sp, "Text field too long"))
    } else if txt.len() == 0 {
        Ok(None)
    } else {
        Ok(Some(txt))
    }
}

// Parse a multipart file field

async fn multipart_file_field<'f>(
    sp: &StaticPages,
    mut field: extract::multipart::Field<'f>,
    max_length: usize,
) -> Result<(Option<String>, Option<Bytes>), (StatusCode, Html<String>)> {
    let file_name = field.file_name().map(|s| s.to_string());

    let mut buffer = BytesMut::with_capacity(32_768);
    let mut space = max_length;

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_err| bad_request(&sp, "Could not read file"))?
    {
        if space < chunk.len() {
            // Consume the remainder of the upload to avoid killing the connection
            // Since we enforce a ContentLengthLimit there is no DOS risk
            while let Ok(Some(_)) = field.chunk().await {}
            return Err(bad_request(&sp, "File size limit exceeded"));
        }
        space -= chunk.len();
        buffer.put(chunk)
    }

    if !buffer.is_empty() {
        Ok((file_name, Some(buffer.freeze())))
    } else {
        Ok((file_name, None))
    }
}

type Submission = extract::ContentLengthLimit<extract::Multipart, FORM_MAX_LENGTH>;

// create_submit: Handler for original post creation forms

async fn create_submit<DB, FR>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    actions: Arc<actions::Actions>,
    db: Arc<DB>,
    fr: Arc<FR>,
    extract::ConnectInfo(addr): extract::ConnectInfo<SocketAddr>,
    extract::Path(board): extract::Path<String>,
    extract::ContentLengthLimit(mut multipart): Submission,
) -> impl IntoResponse
where
    DB: 'static + db::Database + Sync + Send,
    FR: 'static + fr::FileRack + Sync + Send,
{
    let board_id = {
        let pg = unwrap_or_return!(pages.read(), {
            Err(internal_error(&sp, "Could not gain read access to Pages"))
        });

        unwrap_or_return!(pg.board_url_to_id(&board), {
            Ok(response::Redirect::to("/"))
        })
    };

    let mut name = None;
    let mut title = None;
    let mut body = None;
    let mut file_name = None;
    let mut file = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("name") => {
                name = multipart_text_field(&sp, field, 4096).await?;
            },
            Some("title") => {
                title = multipart_text_field(&sp, field, 4096).await?;
            },
            Some("body") => {
                body = multipart_text_field(&sp, field, 16_384).await?;
            },
            Some("file") => {
                (file_name, file) = multipart_file_field(&sp, field, FILE_MAX_SIZE).await?;
            },
            _ => {},
        }
    }

    let file = match file {
        Some(f) => f,
        None => return Err(bad_request(&sp, "You must upload a file")),
    };

    let file_id = unwrap_or_return!(actions.upload_file(fr.as_ref(), file), {
        Err(bad_request(
            &sp,
            "File upload failed - filetype may not be supported",
        ))
    });

    let submission_result = actions.submit_original(
        db.as_ref(),
        board_id,
        addr.ip().to_string(),
        body.unwrap_or_else(|| String::from("")),
        name,
        file_id,
        file_name.unwrap_or_else(|| String::from("")),
        title,
    );

    if let Err(err) = actions.enforce_post_cap(db.as_ref(), fr.as_ref(), board_id) {
        return Err(internal_error(
            &sp,
            &format!("Server failure while enforcing post cap: {:?}", err),
        ));
    }

    match submission_result {
        Ok(_) => Ok(response::Redirect::to(&format!("/{}/catalog", board))),
        Err(_) => Err(internal_error(&sp, "Failed to submit post")),
    }
}

// create_reply: Handler for reply post creation forms

async fn create_reply<DB, FR>(
    sp: Arc<StaticPages>,
    pages: Arc<RwLock<pages::Pages>>,
    actions: Arc<actions::Actions>,
    db: Arc<DB>,
    fr: Arc<FR>,
    extract::ConnectInfo(addr): extract::ConnectInfo<SocketAddr>,
    extract::Path((board, orig_num)): extract::Path<(String, u64)>,
    extract::ContentLengthLimit(mut multipart): Submission,
) -> impl IntoResponse
where
    DB: 'static + db::Database + Sync + Send,
    FR: 'static + fr::FileRack + Sync + Send,
{
    let board_id = {
        let pg = unwrap_or_return!(pages.read(), {
            Err(internal_error(&sp, "Could not gain read access to Pages"))
        });

        unwrap_or_return!(pg.board_url_to_id(&board), {
            Ok(response::Redirect::to("/"))
        })
    };

    let mut name = None;
    let mut body = None;
    let mut file_name = None;
    let mut file = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("name") => {
                name = multipart_text_field(&sp, field, 4096).await?;
            },
            Some("body") => {
                body = multipart_text_field(&sp, field, 16_384).await?;
            },
            Some("file") => {
                (file_name, file) = multipart_file_field(&sp, field, FILE_MAX_SIZE).await?;
            },
            _ => {},
        }
    }

    let mut file_id = None;

    if let Some(file) = file {
        file_id = Some(unwrap_or_return!(actions.upload_file(fr.as_ref(), file), {
            Err(bad_request(
                &sp,
                "File upload failed - filetype may not be supported",
            ))
        }));
    }

    let submission_result = actions.submit_reply(
        db.as_ref(),
        board_id,
        addr.ip().to_string(),
        body.unwrap_or_else(|| String::from("")),
        name,
        file_id,
        file_name,
        orig_num,
    );

    match submission_result {
        Ok(_) => Ok(response::Redirect::to(&format!(
            "/{}/thread/{}",
            board, orig_num
        ))),
        Err(_) => Err(internal_error(&sp, "Failed to submit post")),
    }
}

// Headers for filerack files (necessary to achieve display-in-browser)

fn file_headers(file: &Bytes) -> impl IntoResponseParts {
    [
        (
            "Cache-Control",
            "public, max-age=604800, immutable".to_string(),
        ),
        ("Content-Length", file.len().to_string()),
        ("Content-Type", "image".to_string()),
        ("Content-Disposition", "inline".to_string()),
    ]
}

// files: Handler for full-size filerack files

async fn files<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<FR>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
    FR: 'static + fr::FileRack + Sync + Send,
{
    let file = fr
        .get_file(&file_id)
        .map_err(|_| -> ErrorResponse { not_found(&sp, "No such file").into() })?;
    Ok((StatusCode::OK, file_headers(&file), file))
}

// thumbnails: Handler for thumbnail filerack files

async fn thumbnails<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<FR>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
    FR: 'static + fr::FileRack + Sync + Send,
{
    let file = fr
        .get_file_thumbnail(&file_id)
        .map_err(|_| -> ErrorResponse { not_found(&sp, "No such thumbnail").into() })?;
    Ok((StatusCode::OK, file_headers(&file), file))
}

// not_found: Handler for 404 fallback

async fn route_not_found(sp: Arc<StaticPages>, uri: Uri) -> (StatusCode, impl IntoResponse) {
    not_found(&sp, &format!("404 Not Found ({})", uri))
}

async fn redirect(path: String) -> response::Redirect {
    response::Redirect::to(&path)
}

// Main server method - using tokio runtime

#[tokio::main]
pub async fn serve<DB, FR>(
    config: Config,
    pages: pages::Pages,
    actions: actions::Actions,
    database: DB,
    file_rack: FR,
) where
    DB: 'static + db::Database + Sync + Send,
    FR: 'static + fr::FileRack + Sync + Send,
{
    let sp = StaticPages {
        error_tmpl:   Template::from_file(config.templates_dir.join("error.html.tmpl").as_path())
            .unwrap_or_else(|err| err.die()),
        message_tmpl: Template::from_file(config.templates_dir.join("message.html.tmpl").as_path())
            .unwrap_or_else(|err| err.die()),
    };

    let sp = Arc::new(sp);
    let config = Arc::new(config);

    let pages = Arc::new(RwLock::new(pages));
    let actions = Arc::new(actions);

    let db = Arc::new(database);
    let fr = Arc::new(file_rack);

    let router = Router::new()
        .route(
            "/",
            routing::get({
                let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
                move || homepage(sp, pages, db)
            }),
        )
        .route(
            "/static/*path",
            routing::get({
                let config = config.clone();
                move |path| static_dir(config, path)
            }),
        )
        .route(
            "/:board/thread/:post_num",
            routing::get({
                let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
                move |path| thread(sp, pages, db, path)
            }),
        )
        .route(
            "/:board/",
            routing::get(move |extract::Path(board): extract::Path<String>| {
                redirect(format!("/{}/catalog", board))
            }),
        )
        .route(
            "/:board/catalog",
            routing::get({
                let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
                move |path| catalog(sp, pages, db, path)
            }),
        )
        .route(
            "/:board/create",
            routing::get({
                let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
                move |path| create(sp, pages, db, path)
            }),
        )
        .route(
            "/files/:file_id",
            routing::get({
                let (sp, fr) = (sp.clone(), fr.clone());
                move |path| files(sp, fr, path)
            }),
        )
        .route(
            "/thumbnails/:file_id",
            routing::get({
                let (sp, fr) = (sp.clone(), fr.clone());
                move |path| thumbnails(sp, fr, path)
            }),
        )
        .route(
            "/:board/submit",
            routing::post({
                let (sp, pages, actions, db, fr) = (
                    sp.clone(),
                    pages.clone(),
                    actions.clone(),
                    db.clone(),
                    fr.clone(),
                );
                move |conn, path, form| create_submit(sp, pages, actions, db, fr, conn, path, form)
            }),
        )
        .route(
            "/:board/reply/:orig_num",
            routing::post({
                let (sp, pages, actions, db, fr) = (
                    sp.clone(),
                    pages.clone(),
                    actions.clone(),
                    db.clone(),
                    fr.clone(),
                );
                move |conn, path, form| create_reply(sp, pages, actions, db, fr, conn, path, form)
            }),
        )
        .fallback(
            {
                let sp = sp.clone();
                move |uri| route_not_found(sp, uri)
            }
            .into_service(),
        );

    axum::Server::bind(&config.addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server quit unexpectedly");
}
