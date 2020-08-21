use crate::actions;
use crate::db;
use crate::fr;
use crate::pages;
use crate::template::{Data, Template};
use bytes::{buf::Buf, BufMut, Bytes, BytesMut};
use futures::StreamExt;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex, MutexGuard};
use warp::http::{Response, StatusCode};
use warp::multipart;
use warp::reply;
use warp::reply::Reply;
use warp::{http::Uri, Filter};

type Pages = Arc<Mutex<pages::Pages>>;
type Actions = Arc<Mutex<actions::Actions>>;

lazy_static! {
    static ref ERR_TMPL: Template =
        Template::from_file("templates/error.html.tmpl").unwrap_or_else(|err| err.die());
    static ref MSG_TMPL: Template =
        Template::from_file("templates/message.html.tmpl").unwrap_or_else(|err| err.die());
}

fn error_page(message: &str) -> impl Reply {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    reply::html(ERR_TMPL.render(&Data::new(vals, HashMap::new())))
}

fn message_page(message: &str) -> impl Reply {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    reply::html(MSG_TMPL.render(&Data::new(vals, HashMap::new())))
}

fn acquire_lock<'l, T>(lock: &'l Arc<Mutex<T>>,
                       name: &str)
                       -> Result<MutexGuard<'l, T>, reply::Response> {
    match lock.lock() {
        Ok(guard) => Ok(guard),
        Err(_) => Err(warp::reply::with_status(error_page(&format!("Could not acquire lock: {}",
                                                                   name)),
                                               StatusCode::INTERNAL_SERVER_ERROR).into_response()),
    }
}

enum FormBuffer {
    Empty,
    Overflow,
    Utilised(BytesMut),
}

async fn part_buffer(part: multipart::Part, buf_size: usize) -> FormBuffer {
    let mut chunks = part.stream();
    let mut buffer = BytesMut::with_capacity(buf_size);
    let mut space = buf_size;
    while let Some(Ok(buf)) = chunks.next().await {
        let additional = buf.bytes().len();
        if space < additional {
            return FormBuffer::Overflow;
        }
        buffer.put(buf);
        space -= additional;
    }

    if buffer.len() == 0 {
        FormBuffer::Empty
    } else {
        FormBuffer::Utilised(buffer)
    }
}

// Process multipart bytestream into String
async fn part_string(part: multipart::Part, buf_size: usize) -> Option<String> {
    let buffer = part_buffer(part, buf_size).await;
    let buffer = match buffer {
        FormBuffer::Utilised(bytes) => bytes,
        FormBuffer::Empty | FormBuffer::Overflow => return Some(String::from("")),
    };

    match std::str::from_utf8(&buffer[..]) {
        Ok(s) => Some(String::from(s)),
        Err(_) => None,
    }
}

// Handle multipart POST submission of new thread
async fn create_submit<DB: 'static + db::Database + Sync + Send,
                       FR: 'static + fr::FileRack + Sync + Send>(
    board: String,
    mut data: multipart::FormData,
    p: Pages,
    a: Actions,
    db: Arc<Mutex<DB>>,
    fr: Arc<Mutex<FR>>)
    -> Result<reply::Response, warp::reject::Rejection> {
    let board_id = {
        let p_lock = acquire_lock(&p, "Pages");
        let mut pg = match p_lock {
            Ok(pg) => pg,
            Err(r) => return Ok(r),
        };
        let pages = &mut *pg;

        match pages.board_url_to_id(&board) {
            Some(b_id) => b_id.clone(),
            None => return Ok(warp::redirect(Uri::from_static("/")).into_response()),
        }
    };

    let mut name = None;
    let mut title = None;
    let mut body = None;
    let mut file = FormBuffer::Empty;

    while let Some(Ok(part)) = data.next().await {
        match part.name() {
            "name" => {
                name = part_string(part, 4096).await;
            },
            "title" => {
                title = part_string(part, 4096).await;
            },
            "body" => {
                body = part_string(part, 16384).await;
            },
            "file" => {
                file = part_buffer(part, 524288).await;
            },
            _ => {},
        }
    }

    let file: Bytes = match file {
        FormBuffer::Utilised(bytes) => bytes.freeze(),
        FormBuffer::Overflow => {
            return Ok(message_page("File size limit exceeded").into_response())
        },
        FormBuffer::Empty => return Ok(message_page("You must upload a file").into_response()),
    };

    let a_lock = acquire_lock(&a, "Actions");
    let mut ag = match a_lock {
        Ok(ag) => ag,
        Err(r) => return Ok(r),
    };
    let actions = &mut *ag;

    let file_id = {
        let fr_lock = acquire_lock(&fr, "FileRack");
        let mut rg = match fr_lock {
            Ok(rg) => rg,
            Err(r) => return Ok(r),
        };
        let rack = &mut *rg;

        actions.upload_file(rack, file).unwrap()
    };

    let sub_res = {
        let db_lock = acquire_lock(&db, "Database");
        let mut dg = match db_lock {
            Ok(dg) => dg,
            Err(r) => return Ok(r),
        };
        let database = &mut *dg;

        actions.submit_original(database,
                                board_id,
                                "0.0.0.0".to_string(),
                                body.unwrap(),
                                Some(name.unwrap()),
                                file_id,
                                "yellow_loveless.png".to_string(),
                                Some(title.unwrap()))
    };

    match sub_res {
        Ok(_) => Ok(warp::redirect(format!("/{}/catalog", board).parse::<Uri>().unwrap()).into_response()),
        Err(_) => Err(warp::reject()),
    }
}

async fn create_reply<DB: 'static + db::Database + Sync + Send,
                      FR: 'static + fr::FileRack + Sync + Send>(
    board: String,
    thread: u64,
    mut data: multipart::FormData,
    p: Pages,
    a: Actions,
    db: Arc<Mutex<DB>>,
    fr: Arc<Mutex<FR>>)
    -> Result<reply::Response, warp::reject::Rejection> {
    let board_id = {
        let p_lock = acquire_lock(&p, "Pages");
        let mut pg = match p_lock {
            Ok(pg) => pg,
            Err(r) => return Ok(r),
        };
        let pages = &mut *pg;

        match pages.board_url_to_id(&board) {
            Some(b_id) => b_id.clone(),
            None => return Ok(warp::redirect(Uri::from_static("/")).into_response()),
        }
    };

    let mut name = None;
    let mut body = None;
    let mut file = FormBuffer::Empty;

    while let Some(Ok(part)) = data.next().await {
        match part.name() {
            "name" => {
                name = part_string(part, 4096).await;
            },
            "body" => {
                body = part_string(part, 16384).await;
            },
            "file" => {
                file = part_buffer(part, 524288).await;
            },
            _ => {},
        }
    }

    let a_lock = acquire_lock(&a, "Actions");
    let mut ag = match a_lock {
        Ok(ag) => ag,
        Err(r) => return Ok(r),
    };
    let actions = &mut *ag;

    let file_id = match file {
        FormBuffer::Utilised(bytes) => {
            let file = bytes.freeze();

            let fr_lock = acquire_lock(&fr, "FileRack");
            let mut rg = match fr_lock {
                Ok(rg) => rg,
                Err(r) => return Ok(r),
            };
            let rack = &mut *rg;

            Some(actions.upload_file(rack, file).unwrap())
        },
        FormBuffer::Overflow => {
            return Ok(message_page("File size limit exceeded").into_response())
        },
        FormBuffer::Empty => None,
    };

    let sub_res = {
        let db_lock = acquire_lock(&db, "Database");
        let mut dg = match db_lock {
            Ok(dg) => dg,
            Err(r) => return Ok(r),
        };
        let database = &mut *dg;

        actions.submit_reply(database,
                             board_id,
                             "0.0.0.0".to_string(),
                             body.unwrap(),
                             Some(name.unwrap()),
                             file_id,
                             Some("yellow_loveless.png".to_string()),
                             thread)
    };

    match sub_res {
        Ok(_) => {
            Ok(warp::redirect(format!("/{}/thread/{}", board, thread).parse::<Uri>()
                                                                     .unwrap()).into_response())
        },
        Err(_) => Err(warp::reject()),
    }
}

async fn not_found(_rej: warp::reject::Rejection) -> Result<reply::Response, Infallible> {
    Ok(warp::reply::with_status(message_page("404 Not Found"), StatusCode::NOT_FOUND).into_response())
}

async fn index_redir(_rej: warp::reject::Rejection) -> Result<reply::Response, Infallible> {
    Ok(warp::redirect("/".parse::<Uri>().unwrap()).into_response())
}

// Main server method - using tokio runtime

#[tokio::main]
pub async fn serve<DB: 'static + db::Database + Sync + Send,
                   FR: 'static + fr::FileRack + Sync + Send>(
    pages: pages::Pages,
    actions: actions::Actions,
    database: DB,
    file_rack: FR,
    ip: [u8; 4],
    port: u16) {
    // Wrap pages in Arc<Mutex<>> and move into a filter
    let pages = Arc::new(Mutex::new(pages));
    let pages = warp::any().map(move || pages.clone());

    // Wrap actions in Arc<Mutex<>> and move into a filter
    let actions = Arc::new(Mutex::new(actions));
    let actions = warp::any().map(move || actions.clone());

    // Wrap database in Arc<Mutex<>> and move into a filter
    let database = Arc::new(Mutex::new(database));
    let database = warp::any().map(move || database.clone());

    // Wrap file rack in Arc<Mutex<>> and move into a filter
    let file_rack = Arc::new(Mutex::new(file_rack));
    let file_rack = warp::any().map(move || file_rack.clone());

    // Serve catalog pages
    let catalog =
        warp::path!(String / "catalog").and(pages.clone())
                                       .and(database.clone())
                                       .map(|board: String, p: Pages, db: Arc<Mutex<DB>>| {
                                           let pages = &mut (*p.lock().unwrap());
                                           if let Some(board_id) = pages.board_url_to_id(&board) {
                                               let database = &(*db.lock().unwrap());
                                               let page_ref = pages::PageRef::Catalog(*board_id);
                                               let page = pages.get_page(database, &page_ref)
                                                               .unwrap()
                                                               .page_text
                                                               .to_string();

                                               Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(page)
                                           } else {
                                               Response::builder().status(404)
                                                                  .body("Not Found".to_string())
                                           }
                                       });

    // Serve thread pages
    let thread = warp::path!(String / "thread" / u64)
        .and(pages.clone())
        .and(database.clone())
        .map(
            |board: String, orig_num: u64, p: Pages, db: Arc<Mutex<DB>>| {
                let pages = &mut (*p.lock().unwrap());

                if let Some(board_id) = pages.board_url_to_id(&board) {
                    let database = &(*db.lock().unwrap());
                    let page_ref = pages::PageRef::Thread(*board_id, orig_num);
                    let page = pages.get_page(database, &page_ref);

                    match page {
                        Ok(page) => Response::builder()
                            .header("Content-Type", "text/html; charset=utf-8")
                            .body(page.page_text.to_string()),
                        Err(_) => Response::builder()
                            .status(404)
                            .body("Not Found".to_string()),
                    }
                } else {
                    Response::builder()
                        .status(404)
                        .body("Not Found".to_string())
                }
            },
        );

    // Serve thread creation page
    let create =
        warp::path!(String / "create").and(pages.clone())
                                      .and(database.clone())
                                      .map(|board: String, p: Pages, db: Arc<Mutex<DB>>| {
                                          let pages = &mut (*p.lock().unwrap());
                                          if let Some(board_id) = pages.board_url_to_id(&board) {
                                              let database = &(*db.lock().unwrap());
                                              let page_ref = pages::PageRef::Create(*board_id);
                                              let page = pages.get_page(database, &page_ref)
                                                              .unwrap()
                                                              .page_text
                                                              .to_string();

                                              Response::builder().header("Content-Type",
                                                                         "text/html; charset=utf-8")
                                                                 .body(page)
                                          } else {
                                              Response::builder().status(404)
                                                                 .body("Not Found".to_string())
                                          }
                                      });

    // Serve submit action
    let submit = warp::path!(String / "submit").and(warp::multipart::form().and(pages.clone())
                                                                           .and(actions.clone())
                                                                           .and(database.clone())
                                                                           .and(file_rack.clone()))
                                               .and_then(create_submit);

    // Serve reply action
    let reply =
        warp::path!(String / "reply" / u64).and(warp::multipart::form().and(pages.clone())
                                                                       .and(actions.clone())
                                                                       .and(database.clone())
                                                                       .and(file_rack.clone()))
                                           .and_then(create_reply);

    // Serve rack files
    let files = warp::path!("files" / String).and(file_rack.clone())
                                             .map(|file_id: String, fr: Arc<Mutex<FR>>| {
                                                 let file_rack = &mut (*fr.lock().unwrap());
                                                 match file_rack.get_file(&file_id) {
                Ok(file) => Response::builder()
                    .header("Cache-Control", "public, max-age=604800, immutable")
                    .body(file),
                Err(_err) => Response::builder()
                    .status(404)
                    .body(Bytes::from("Not Found")),
            }
                                             });

    // Serve static resources
    let stat = warp::path("static").and(warp::fs::dir("./static"));

    // Bundle routes together and run
    let routes = warp::get().and(stat.or(files).or(catalog).or(thread).or(create))
                            .or(warp::post().and(submit.or(reply).unify().recover(index_redir)))
                            .recover(not_found);

    warp::serve(routes).run((ip, port)).await;
}
