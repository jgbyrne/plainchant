use crate::actions;
use crate::db;
use crate::fr;
use crate::pages;
use crate::template::{Data, Template};
use crate::Config;

use axum::http::{StatusCode, Uri};
use axum::{body, extract, response, routing, Extension, Router};
use axum::response::{IntoResponse, IntoResponseParts, ErrorResponse};
use axum::handler::Handler;

use tokio;
use tokio_util::io::ReaderStream;

use mime_guess;
use bytes::Bytes;

use std::collections::HashMap;
use std::path;
use std::sync::{Arc, Mutex};
use std::ops::DerefMut;

// More complex templates are handled by Pages, but these
// simple message pages we can handle directly
struct StaticPages {
    error_tmpl:   Template,
    message_tmpl: Template,
}

fn error_page(sp: &StaticPages, message: &str) -> response::Html<String> {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    response::Html::from(
        sp.error_tmpl 
            .render(&Data::values(vals)),
    )
}

fn message_page(sp: &StaticPages, message: &str) -> response::Html<String> {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    response::Html::from(
        sp.message_tmpl 
            .render(&Data::values(vals)),
    )
}

async fn static_dir(
    config: Arc<Config>,
    extract::Path(path): extract::Path<path::PathBuf>,
) -> impl IntoResponse {
    let path = path.strip_prefix("/").unwrap();
    let full_path = config.static_dir.join(&path);

    let file = match tokio::fs::File::open(&full_path).await {
        Ok(f) => f,
        Err(err) => {
            return Err((
                StatusCode::NOT_FOUND,
                format!("Could not retrieve static file: {}", path.to_string_lossy()),
            ))
        },
    };

    let stream = ReaderStream::new(file);
    let body = body::StreamBody::new(stream);

    let mime = mime_guess::from_path(&full_path).first_or_octet_stream();

    let headers = response::AppendHeaders([("Content-Type", mime.to_string())]);

    Ok((headers, body))
}

async fn thread<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    db: Arc<DB>,
    extract::Path((board, post_num)): extract::Path<(String, u64)>,
) -> Result<(StatusCode, response::Html<String>), ErrorResponse>
where
     DB: 'static + db::Database + Sync + Send
{
    match pages.lock() {
        Ok(mut guard) => {
            let mut pages = guard.deref_mut();
            if let Some(board_id) = pages.board_url_to_id(&board) {
                let page_ref = pages::PageRef::Thread(board_id, post_num);

                match pages.get_page(db.as_ref(), &page_ref) {
                    Ok(page) => {
                        Ok((StatusCode::OK,
                            response::Html::from(page.page_text.to_string())))
                    },
                    Err(_) => {
                        // The board exists but the original post does not
                        // Let's try and fetch it as a reply
                        match db.get_reply(board_id, post_num) {
                            Ok(reply) => {
                                let uri = format!(
                                    "/{}/thread/{}#{}",
                                    &board,
                                    reply.orig_num,
                                    post_num,
                                );
                                Err(response::Redirect::permanent(&uri).into())
                            },
                            Err(_) => {
                                Ok((StatusCode::NOT_FOUND,
                                 message_page(sp.as_ref(), "No such thread")))
                            },
                        }
                    },
                }

            }
            else {
                Ok((StatusCode::NOT_FOUND,
                 message_page(sp.as_ref(), "No such board")))
            }
        },
        Err(err) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR,
             error_page(sp.as_ref(), &format!("Could not acquire lock on pages"))))
        },
    }
}


async fn catalog<DB>(
    sp: Arc<StaticPages>,
    pages: Arc<Mutex<pages::Pages>>,
    db: Arc<DB>,
    extract::Path(board): extract::Path<String>,
) -> (StatusCode, response::Html<String>) 
where
     DB: 'static + db::Database + Sync + Send
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

                (StatusCode::OK, response::Html::from(page))
            }
            else {
                (StatusCode::NOT_FOUND,
                 message_page(sp.as_ref(), "No such board"))
            }
        },
        Err(err) => {
            (StatusCode::INTERNAL_SERVER_ERROR,
             error_page(sp.as_ref(), "Could not acquire lock on pages")) 
        },
    }
}

fn file_headers(file: &Bytes) -> impl IntoResponseParts {
    [ 
        ("Cache-Control", "public, max-age=604800, immutable".to_string()),
        ("Content-Length", file.len().to_string()),
        ("Content-Type", "image".to_string()),
        ("Content-Disposition", "inline".to_string())
    ]
}

async fn files<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<Mutex<FR>>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
     FR: 'static + fr::FileRack + Sync + Send
{
    match fr.lock() {
        Ok(mut guard) => {
            let file_rack = guard.deref_mut();

            match file_rack.get_file(&file_id) {
                Ok(file) => {
                    Ok((
                        StatusCode::OK,
                        file_headers(&file),
                        file,
                    ))
                },
                Err(_) => {
                    Err((
                        StatusCode::NOT_FOUND,
                        message_page(sp.as_ref(), "No such file"),
                    ).into())
                },
            }
        },
        Err(err) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR,
                error_page(sp.as_ref(), "Could not acquire lock on filerack")).into())
        }
    }
}

async fn thumbnails<FR>(
    sp: Arc<StaticPages>,
    fr: Arc<Mutex<FR>>,
    extract::Path(file_id): extract::Path<String>,
) -> Result<(StatusCode, impl IntoResponseParts, Bytes), ErrorResponse>
where
     FR: 'static + fr::FileRack + Sync + Send
{
    match fr.lock() {
        Ok(mut guard) => {
            let file_rack = guard.deref_mut();

            match file_rack.get_file_thumbnail(&file_id) {
                Ok(file) => {
                    Ok((
                        StatusCode::OK,
                        file_headers(&file),
                        file,
                    ))
                },
                Err(_) => {
                    Err((
                        StatusCode::NOT_FOUND,
                        message_page(sp.as_ref(), "No such thumbnail"),
                    ).into())
                },
            }
        },
        Err(err) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR,
                error_page(sp.as_ref(), "Could not acquire lock on filerack")).into())
        }
    }
}

async fn not_found(sp: Arc<StaticPages>, uri: Uri) -> (StatusCode, impl IntoResponse) {
    (StatusCode::NOT_FOUND, message_page(sp.as_ref(), "404 Not Found"))
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
        .route("/static/*path", routing::get({
            let config = config.clone();
            move |path| static_dir(config, path)
        }))
        .route("/:board/thread/:post_num", routing::get({
            let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
            move |path| thread(sp, pages, db, path)
        }))
        .route("/:board/catalog", routing::get({
            let (sp, pages, db) = (sp.clone(), pages.clone(), db.clone());
            move |path| catalog(sp, pages, db, path)
        }))
        .route("/files/:file_id", routing::get({
            let (sp, fr) = (sp.clone(), fr.clone());
            move |path| files(sp, fr, path)
        }))
        .route("/thumbnails/:file_id", routing::get({
            let (sp, fr) = (sp.clone(), fr.clone());
            move |path| thumbnails(sp, fr, path)
        }))
        .fallback({
            let sp = sp.clone();
            move |uri| not_found(sp, uri)
        }.into_service());

    axum::Server::bind(&config.addr)
        .serve(router.into_make_service())
        .await
        .expect("Server quit unexpectedly");
}

