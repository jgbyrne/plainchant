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

// This value is equivalent to 64 MiB in bytes
// Right now, above this, the browser is returned nothing rather than a proper error page
// Therefore we make this value unreasonably high so 99% of the time a proper error is shown
static FORM_MAX_LENGTH: u64 = 67_108_864;

// Alias these types to make some code less ugly
type Ptr<T> = Arc<Mutex<T>>;

// More complex templates are handled by Pages, but these
// simple message pages we can handle directly
lazy_static! {
    static ref ERR_TMPL: Template =
        Template::from_file("templates/error.html.tmpl").unwrap_or_else(|err| err.die());
    static ref MSG_TMPL: Template =
        Template::from_file("templates/message.html.tmpl").unwrap_or_else(|err| err.die());
}

// Produce an Error page Reply
fn error_page(message: &str) -> impl Reply {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    reply::html(ERR_TMPL.render(&Data::new(vals, HashMap::new(), HashMap::new())))
}

// Produce a Message page Reply
fn message_page(message: &str) -> impl Reply {
    let mut vals = HashMap::new();
    vals.insert(String::from("message"), String::from(message));
    reply::html(MSG_TMPL.render(&Data::new(vals, HashMap::new(), HashMap::new())))
}

// Lock an Arc<Mutex<>>, returning a rendered error page if this fails
fn acquire_lock<'l, T>(lock: &'l Ptr<T>, name: &str) -> Result<MutexGuard<'l, T>, reply::Response> {
    match lock.lock() {
        Ok(guard) => Ok(guard),
        Err(_) => Err(warp::reply::with_status(error_page(&format!("Could not acquire lock: {}",
                                                                   name)),
                                               StatusCode::INTERNAL_SERVER_ERROR).into_response()),
    }
}

// A buffer read from one part of a Multipart Form
enum FormBuffer {
    Empty,
    Overflow,
    Utilised(BytesMut),
}

// Process multipart bytestream into Buffer
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

    if buffer.is_empty() {
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
    p: Ptr<pages::Pages>,
    a: Ptr<actions::Actions>,
    db: Ptr<DB>,
    fr: Ptr<FR>)
    -> Result<reply::Response, warp::reject::Rejection> {
    // Look-up Board URL to retrieve ID
    let board_id = {
        let p_lock = acquire_lock(&p, "Pages");
        let mut pg = match p_lock {
            Ok(pg) => pg,
            Err(r) => return Ok(r),
        };
        let pages = &mut *pg;

        match pages.board_url_to_id(&board) {
            Some(b_id) => *b_id,
            None => return Ok(warp::redirect(Uri::from_static("/")).into_response()),
        }
    };

    // Extract each field from the form
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

    // Handle potential outcomes of reading the file
    let file: Bytes = match file {
        FormBuffer::Utilised(bytes) => bytes.freeze(),
        FormBuffer::Overflow => {
            return Ok(message_page("File size limit exceeded").into_response())
        },
        FormBuffer::Empty => return Ok(message_page("You must upload a file").into_response()),
    };

    // Obtain a lock on Actions
    let a_lock = acquire_lock(&a, "Actions");
    let mut ag = match a_lock {
        Ok(ag) => ag,
        Err(r) => return Ok(r),
    };
    let actions = &mut *ag;

    // Upload the file to the FileRack
    let file_id = {
        let fr_lock = acquire_lock(&fr, "FileRack");
        let mut rg = match fr_lock {
            Ok(rg) => rg,
            Err(r) => return Ok(r),
        };
        let rack = &mut *rg;

        match actions.upload_file(rack, file) {
            Ok(file_id) => file_id,
            Err(_) => return Ok(message_page("File upload failed - filetype may not be supported").into_response()),
        }
    };

    // Submit the post to the Database
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
                                body.unwrap_or_else(|| String::from("")),
                                Some(name.unwrap_or_else(|| String::from(""))),
                                file_id,
                                "yellow_loveless.png".to_string(),
                                Some(title.unwrap_or_else(|| String::from(""))))
    };

    // Handle the outcome of the submission
    match sub_res {
        Ok(_) => Ok(warp::redirect(format!("/{}/catalog", board).parse::<Uri>().expect("Could not parse catalog Uri")).into_response()),
        Err(_) => Err(warp::reject()),
    }
}

// Handle multipart submission of thread reply
async fn create_reply<DB: 'static + db::Database + Sync + Send,
                      FR: 'static + fr::FileRack + Sync + Send>(
    board: String,
    thread: u64,
    mut data: multipart::FormData,
    p: Ptr<pages::Pages>,
    a: Ptr<actions::Actions>,
    db: Ptr<DB>,
    fr: Ptr<FR>)
    -> Result<reply::Response, warp::reject::Rejection> {
    // Look-up board ID from URL
    let board_id = {
        let p_lock = acquire_lock(&p, "Pages");
        let mut pg = match p_lock {
            Ok(pg) => pg,
            Err(r) => return Ok(r),
        };
        let pages = &mut *pg;

        match pages.board_url_to_id(&board) {
            Some(b_id) => *b_id,
            None => return Ok(warp::redirect(Uri::from_static("/")).into_response()),
        }
    };

    // Extract fields of multipart form
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

    // Acquire lock on Actions
    let a_lock = acquire_lock(&a, "Actions");
    let mut ag = match a_lock {
        Ok(ag) => ag,
        Err(r) => return Ok(r),
    };
    let actions = &mut *ag;

    // Handle different statuses of uploaded file
    let file_id = match file {
        FormBuffer::Utilised(bytes) => {
            let file = bytes.freeze();

            let fr_lock = acquire_lock(&fr, "FileRack");
            let mut rg = match fr_lock {
                Ok(rg) => rg,
                Err(r) => return Ok(r),
            };
            let rack = &mut *rg;

            match actions.upload_file(rack, file) {
                Ok(file_id) => Some(file_id),
                Err(_) => return Ok(message_page("File upload failed - filetype may not be supported").into_response()),
            }
        },
        FormBuffer::Overflow => {
            return Ok(message_page("File size limit exceeded").into_response())
        },
        FormBuffer::Empty => None,
    };

    // Submit reply to Database
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
                             body.unwrap_or_else(|| String::from("")),
                             Some(name.unwrap_or_else(|| String::from(""))),
                             file_id,
                             Some("yellow_loveless.png".to_string()),
                             thread)
    };

    // Handle outcome of submission
    match sub_res {
        Ok(_) => {
            Ok(warp::redirect(format!("/{}/thread/{}", board, thread)
                                        .parse::<Uri>()
                                        .expect("Could not parse thread Uri"))
                     .into_response())
        },
        Err(_) => Err(warp::reject()),
    }
}

// Some handlers for common responses

async fn not_found(_rej: warp::reject::Rejection) -> Result<reply::Response, Infallible> {
    Ok(warp::reply::with_status(message_page("404 Not Found"), StatusCode::NOT_FOUND).into_response())
}

async fn index_redir(_rej: warp::reject::Rejection) -> Result<reply::Response, Infallible> {
    Ok(warp::redirect("/".parse::<Uri>().expect("Could not parse root Uri")).into_response())
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
                                       .map(|board: String, p: Ptr<pages::Pages>, db: Ptr<DB>| {
                                           // Acquire lock on Pages
                                           let p_lock = acquire_lock(&p, "Pages");
                                           let mut pg = match p_lock {
                                               Ok(pg) => pg,
                                               Err(r) => return Ok(r),
                                           };
                                           let pages = &mut *pg;

                                           // Retrieve Catalog page for board
                                           if let Some(board_id) = pages.board_url_to_id(&board) {
                                               let db_lock = acquire_lock(&db, "Database");
                                               let mut dg = match db_lock {
                                                   Ok(dg) => dg,
                                                   Err(r) => return Ok(r),
                                               };
                                               let database = &mut *dg;

                                               let page_ref = pages::PageRef::Catalog(*board_id);
                                               let page = pages.get_page(database, &page_ref)
                                                               .expect("Could not access catalog for extant board")
                                                               .page_text
                                                               .to_string();

                                               Ok(reply::with_header(reply::html(page), "Content-Type", "text/html; charset=utf-8").into_response())
                                           } else {
                                               Ok(warp::reply::with_status(message_page("No such board"), StatusCode::NOT_FOUND).into_response())
                                           }
                                       });

    // Serve thread pages
    let thread = warp::path!(String / "thread" / u64)
        .and(pages.clone())
        .and(database.clone())
        .map(
            |board: String, orig_num: u64, p: Ptr<pages::Pages>, db: Arc<Mutex<DB>>| {
               // Acquire lock on Pages
               let p_lock = acquire_lock(&p, "Pages");
               let mut pg = match p_lock {
                   Ok(pg) => pg,
                   Err(r) => return Ok(r),
               };
               let pages = &mut *pg;

               // Retreive Thread
               if let Some(board_id) = pages.board_url_to_id(&board) {
                   let db_lock = acquire_lock(&db, "Database");
                   let mut dg = match db_lock {
                       Ok(dg) => dg,
                       Err(r) => return Ok(r),
                   };
                   let database = &mut *dg;

                   let page_ref = pages::PageRef::Thread(*board_id, orig_num);
                   let page = pages.get_page(database, &page_ref);

                   match page {
                       Ok(page) => Ok(reply::with_header(reply::html(page.page_text.to_string()), "Content-Type", "text/html; charset=utf-8").into_response()),
                       Err(_) => Ok(warp::reply::with_status(message_page("No such thread"), StatusCode::NOT_FOUND).into_response()),
                   }
               } else {
                   Ok(warp::reply::with_status(message_page("No such board"), StatusCode::NOT_FOUND).into_response())
               }
            },
        );

    // Serve thread creation page
    let create =
        warp::path!(String / "create").and(pages.clone())
                                      .and(database.clone())
                                      .map(|board: String, p: Ptr<pages::Pages>, db: Ptr<DB>| {
                                           // Acquire lock on Pages
                                           let p_lock = acquire_lock(&p, "Pages");
                                           let mut pg = match p_lock {
                                               Ok(pg) => pg,
                                               Err(r) => return Ok(r),
                                           };
                                           let pages = &mut *pg;

                                           // Retrieve thread creation page for board
                                           if let Some(board_id) = pages.board_url_to_id(&board) {
                                              let db_lock = acquire_lock(&db, "Database");
                                              let mut dg = match db_lock {
                                                  Ok(dg) => dg,
                                                  Err(r) => return Ok(r),
                                              };
                                              let database = &mut *dg;

                                              let page_ref = pages::PageRef::Create(*board_id);
                                              let page = pages.get_page(database, &page_ref)
                                                              .expect("Could not access thread creation page for extant board")
                                                              .page_text
                                                              .to_string();

                                              Ok(reply::with_header(reply::html(page), "Content-Type", "text/html; charset=utf-8").into_response())
                                           } else {
                                              Ok(warp::reply::with_status(message_page("No such board"), StatusCode::NOT_FOUND).into_response())
                                           }
                                      });

    // Serve submit action
    let submit =
        warp::path!(String / "submit").and(warp::multipart::form().max_length(FORM_MAX_LENGTH)
                                                                  .and(pages.clone())
                                                                  .and(actions.clone())
                                                                  .and(database.clone())
                                                                  .and(file_rack.clone()))
                                      .and_then(create_submit);

    // Serve reply action
    let reply =
        warp::path!(String / "reply" / u64).and(warp::multipart::form().max_length(FORM_MAX_LENGTH)
                                                                       .and(pages.clone())
                                                                       .and(actions.clone())
                                                                       .and(database.clone())
                                                                       .and(file_rack.clone()))
                                           .and_then(create_reply);

    // Serve rack files
    let files = warp::path!("files" / String).and(file_rack.clone())
                                             .map(|file_id: String, fr: Ptr<FR>| {
                                                 // Lock file rack
                                                 let fr_lock = acquire_lock(&fr, "FileRack");
                                                 let mut rg = match fr_lock {
                                                     Ok(rg) => rg,
                                                     Err(r) => return Ok(r),
                                                 };
                                                 let file_rack = &mut *rg;

                                                 // Retrieve file from rack
                                                 match file_rack.get_file(&file_id) {
                                                    Ok(file) => Ok(Response::builder()
                                                                    .header("Cache-Control", "public, max-age=604800, immutable")
                                                                    .body(file).expect("Failed to build file response").into_response()),
                                                    Err(_err) => Ok(Response::builder()
                                                                    .status(404)
                                                                    .body(Bytes::from("Not Found"))
                                                                    .expect("Failed to build 404 response").into_response()),
                                                 }
                                             });

    // Serve thumbnail files
    // This is a bit more code duplication than I'd like, but
    // it's preferable to the wrong abstraction :-)
    let thumbnails = warp::path!("thumbnails" / String).and(file_rack.clone())
                                             .map(|file_id: String, fr: Ptr<FR>| {
                                                 // Lock file rack
                                                 let fr_lock = acquire_lock(&fr, "FileRack");
                                                 let mut rg = match fr_lock {
                                                     Ok(rg) => rg,
                                                     Err(r) => return Ok(r),
                                                 };
                                                 let file_rack = &mut *rg;

                                                 // Retrieve file thumbnail from rack
                                                 match file_rack.get_file_thumbnail(&file_id) {
                                                    Ok(file) => Ok(Response::builder()
                                                                    .header("Cache-Control", "public, max-age=604800, immutable")
                                                                    .body(file).expect("Failed to build file response").into_response()),
                                                    Err(_err) => Ok(Response::builder()
                                                                    .status(404)
                                                                    .body(Bytes::from("Not Found"))
                                                                    .expect("Failed to build 404 response").into_response()),
                                                 }
                                             });

    // Serve static resources
    let stat = warp::path("static").and(warp::fs::dir("./static"));

    // Bundle routes together and run
    let routes = warp::get().and(stat.or(files)
                                     .or(thumbnails)
                                     .or(catalog)
                                     .or(thread)
                                     .or(create))
                            .or(warp::post().and(submit.or(reply).unify().recover(index_redir)))
                            .recover(not_found);

    warp::serve(routes).run((ip, port)).await;
}
