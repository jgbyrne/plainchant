use crate::actions;
use crate::db;
use crate::fr;
use crate::pages;
use crate::template::{Data, Template};
use crate::Config;

use axum::handler::Handler;
use axum::http::{StatusCode, Uri};
use axum::response::{ErrorResponse, Html, IntoResponse, IntoResponseParts};
use axum::{body, extract, response, routing, Extension, Router};

use tokio;
use tokio_util::io::ReaderStream;

use bytes::{BufMut, Bytes, BytesMut};
use mime_guess;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::path;
use std::sync::{Arc, Mutex};

// This value is equivalent to 64 MiB in bytes;
const FORM_MAX_LENGTH: u64 = 67_108_864;

// More complex templates are handled by Pages, but these
// simple message pages we can handle directly
struct StaticPages {
    error_tmpl:   Template,
    message_tmpl: Template,
}

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
        Err(err) => Err((
            StatusCode::NOT_FOUND,
            format!("Could not retrieve static file: {}", path.to_string_lossy()),
        )),
    }
}

async fn thread<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    db: Arc<DB>,
    extract::Path((board, post_num)): extract::Path<(String, u64)>,
) -> Result<(StatusCode, Html<String>), ErrorResponse>
where
    DB: 'static + db::Database + Sync + Send,
{
    match pages.lock() {
        Ok(mut guard) => {
            let mut pages = guard.deref_mut();
            if let Some(board_id) = pages.board_url_to_id(&board) {
                let page_ref = pages::PageRef::Thread(board_id, post_num);

                match pages.get_page(db.as_ref(), &page_ref) {
                    Ok(page) => Ok((StatusCode::OK, Html::from(page.page_text.to_string()))),
                    Err(_) => {
                        // The board exists but the original post does not
                        // Let's try and fetch it as a reply
                        match db.get_reply(board_id, post_num) {
                            Ok(reply) => {
                                let uri =
                                    format!("/{}/thread/{}#{}", &board, reply.orig_num, post_num,);
                                Err(response::Redirect::permanent(&uri).into())
                            },
                            Err(_) => Ok((
                                StatusCode::NOT_FOUND,
                                message_page(sp.as_ref(), "No such thread"),
                            )),
                        }
                    },
                }
            } else {
                Ok((
                    StatusCode::NOT_FOUND,
                    message_page(sp.as_ref(), "No such board"),
                ))
            }
        },
        Err(err) => Ok(internal_error(
            sp.as_ref(),
            &format!("Could not acquire lock on pages"),
        )),
    }
}

async fn catalog<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    db: Arc<DB>,
    extract::Path(board): extract::Path<String>,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    match pages.lock() {
        Ok(mut guard) => {
            let pages = guard.deref_mut();
            if let Some(board_id) = pages.board_url_to_id(&board) {
                let page_ref = pages::PageRef::Catalog(board_id);
                let page = pages
                    .get_page(db.as_ref(), &page_ref)
                    .expect("Could not access catalog for extant board")
                    .page_text
                    .to_string();

                (StatusCode::OK, Html::from(page))
            } else {
                (
                    StatusCode::NOT_FOUND,
                    message_page(sp.as_ref(), "No such board"),
                )
            }
        },
        Err(err) => internal_error(sp.as_ref(), "Could not acquire lock on pages"),
    }
}

async fn create<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    db: Arc<DB>,
    extract::Path(board): extract::Path<String>,
) -> (StatusCode, Html<String>)
where
    DB: 'static + db::Database + Sync + Send,
{
    match pages.lock() {
        Ok(mut guard) => {
            let pages = guard.deref_mut();
            if let Some(board_id) = pages.board_url_to_id(&board) {
                let page_ref = pages::PageRef::Create(board_id);
                let page = pages
                    .get_page(db.as_ref(), &page_ref)
                    .expect("Could not access thread creation page for extant board")
                    .page_text
                    .to_string();
                (StatusCode::OK, Html(page))
            } else {
                (
                    StatusCode::NOT_FOUND,
                    message_page(sp.as_ref(), "No such board"),
                )
            }
        },
        Err(err) => internal_error(sp.as_ref(), "Could not acquire lock on pages"),
    }
}

type Submission = extract::ContentLengthLimit<extract::Multipart, FORM_MAX_LENGTH>;

async fn create_submit<DB, FR>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    actions: Arc<actions::Actions>,
    db: Arc<DB>,
    fr: Arc<Mutex<FR>>,
    extract::ConnectInfo(addr): extract::ConnectInfo<SocketAddr>,
    extract::Path(board): extract::Path<String>,
    extract::ContentLengthLimit(mut multipart): Submission,
) -> impl IntoResponse
where
    DB: 'static + db::Database + Sync + Send,
    FR: 'static + fr::FileRack + Sync + Send,
{
    let board_id = {
        match pages.lock() {
            Ok(guard) => match guard.deref().board_url_to_id(&board) {
                Some(board_id) => board_id,
                None => {
                    return Ok(response::Redirect::to("/"));
                },
            },
            Err(err) => {
                return Err(internal_error(
                    sp.as_ref(),
                    "Could not acquire lock on pages",
                ));
            },
        }
    };

    let mut name = None;
    let mut title = None;
    let mut body = None;
    let mut file_name = None;
    let mut file = None;

    while let Ok(Some(mut field)) = multipart.next_field().await {
        match field.name() {
            Some("name") => {
                name = Some(
                    field
                        .text()
                        .await
                        .map_err(|err| bad_request(sp.as_ref(), "Could not parse text field"))?,
                );
            },
            Some("title") => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|err| bad_request(sp.as_ref(), "Could not parse title field"))?,
                );
            },
            Some("body") => {
                body = Some(
                    field
                        .text()
                        .await
                        .map_err(|err| bad_request(sp.as_ref(), "Could not parse body field"))?,
                );
            },
            Some("file") => {
                file_name = field.file_name().map(|s| s.to_string());

                let mut buffer = BytesMut::with_capacity(32_768);
                let mut space = 524_288;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|err| bad_request(sp.as_ref(), "Could not read file"))?
                {
                    if space < chunk.len() {
                        while let Ok(Some(_)) = field.chunk().await {}
                        return Err(bad_request(sp.as_ref(), "File size limit exceeded"));
                    }
                    space -= chunk.len();
                    buffer.put(chunk)
                }

                if !buffer.is_empty() {
                    file = Some(buffer.freeze());
                }
            },
            _ => {},
        }
    }

    let file = match file {
        Some(f) => f,
        None => return Err(bad_request(sp.as_ref(), "You must upload a file")),
    };

    match fr.lock() {
        Ok(mut guard) => {
            let mut file_rack = guard.deref_mut();
            let file_id = match actions.upload_file(file_rack, file) {
                Ok(id) => id,
                Err(_) => {
                    return Err(bad_request(
                        sp.as_ref(),
                        "File upload failed - filetype may not be supported",
                    ));
                },
            };

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

            if let Err(err) = actions.enforce_post_cap(db.as_ref(), file_rack, board_id) {
                return Err(internal_error(
                    sp.as_ref(),
                    "Server failure while enforcing post cap",
                ));
            }

            match submission_result {
                Ok(_) => Ok(response::Redirect::to(&format!("/{}/catalog", board))),
                Err(_) => Err(internal_error(sp.as_ref(), "Failed to submit post")),
            }
        },
        Err(err) => Err(internal_error(
            sp.as_ref(),
            "Could not obtain lock for filerack",
        )),
    }
}

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

async fn files<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<Mutex<FR>>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
    FR: 'static + fr::FileRack + Sync + Send,
{
    match fr.lock() {
        Ok(mut guard) => {
            let file_rack = guard.deref_mut();

            match file_rack.get_file(&file_id) {
                Ok(file) => Ok((StatusCode::OK, file_headers(&file), file)),
                Err(_) => Err((
                    StatusCode::NOT_FOUND,
                    message_page(sp.as_ref(), "No such file"),
                )
                    .into()),
            }
        },
        Err(err) => Err(internal_error(sp.as_ref(), "Could not acquire lock on filerack").into()),
    }
}

async fn thumbnails<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<Mutex<FR>>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
    FR: 'static + fr::FileRack + Sync + Send,
{
    match fr.lock() {
        Ok(mut guard) => {
            let file_rack = guard.deref_mut();

            match file_rack.get_file_thumbnail(&file_id) {
                Ok(file) => Ok((StatusCode::OK, file_headers(&file), file)),
                Err(_) => Err((
                    StatusCode::NOT_FOUND,
                    message_page(sp.as_ref(), "No such thumbnail"),
                )
                    .into()),
            }
        },
        Err(err) => Err(internal_error(sp.as_ref(), "Could not acquire lock on filerack").into()),
    }
}

async fn not_found(sp: Arc<StaticPages>, uri: Uri) -> (StatusCode, impl IntoResponse) {
    (
        StatusCode::NOT_FOUND,
        message_page(sp.as_ref(), "404 Not Found"),
    )
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

    let pages = Arc::new(Mutex::new(pages));
    let actions = Arc::new(actions);

    let db = Arc::new(database);
    let fr = Arc::new(Mutex::new(file_rack));

    let router = Router::new()
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
        .fallback(
            {
                let sp = sp.clone();
                move |uri| not_found(sp, uri)
            }
            .into_service(),
        );

    axum::Server::bind(&config.addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server quit unexpectedly");
}
