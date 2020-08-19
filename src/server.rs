use crate::db;
use crate::fr;
use crate::pages;
use crate::actions;
use crate::util;
use std::convert::Infallible;
use warp::hyper::Body;
use warp::{http::Uri, Filter};
use warp::multipart;
use warp::reply::Reply;
use warp::http::Response;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures::StreamExt;
use bytes::{Bytes, BytesMut, BufMut, buf::Buf};
use std::ascii;

type Pages = Arc<Mutex<pages::Pages>>;
type Actions = Arc<Mutex<actions::Actions>>;

async fn part_buffer(part: multipart::Part, buf_size: usize) -> Option<BytesMut> {
    let mut chunks = part.stream();
    let mut buffer = BytesMut::with_capacity(buf_size);
    while let Some(Ok(buf)) = chunks.next().await {
        buffer.put(buf); 
    }
    Some(buffer)
}

// Process multipart bytestream into String
async fn part_string(part: multipart::Part, buf_size: usize) -> Option<String> {
    let buffer = part_buffer(part, buf_size).await?;
    match std::str::from_utf8(&buffer[..]) {
        Ok(s) => Some(String::from(s)),
        Err(_) => None,
    }
}

// Handle multipart POST submission of new thread
async fn create_submit<DB: 'static + db::Database+Sync+Send,
                       FR: 'static + fr::FileRack+Sync+Send>
                      (board: String, mut data: multipart::FormData,
                       p: Pages, a: Actions,
                       db: Arc<Mutex<DB>>, fr: Arc<Mutex<FR>>) -> Result<impl warp::Reply, Infallible> {
    let board_id = {
        let mut pages = p.lock().unwrap();
        match pages.board_url_to_id(&board) {
            Some(b_id) => b_id.clone(),
            None => return Ok(warp::redirect(Uri::from_static("/"))),
        }
    };
    
    let mut name = None;
    let mut title = None;
    let mut body = None;
    let mut file = None;
    while let Some(Ok(mut part)) = data.next().await {
       match part.name() {
            "name"  => {
                name = part_string(part, 4096).await;
            },
            "title" => {
                title = part_string(part, 4096).await;
            },
            "body"  => {
                body = part_string(part, 16384).await;
            },
            "file"  => {
                file = part_buffer(part, 524288).await;
            },
            _ => {},
        }
    }
    let mut actions = a.lock().unwrap();

    let file_id = actions.upload_file(&mut *fr.lock().unwrap(), file.unwrap().freeze()).unwrap();

    actions.submit_original(&mut *db.lock().unwrap(),
                            board_id, "0.0.0.0".to_string(),
                            body.unwrap(),
                            Some(name.unwrap()),
                            file_id,
                            "yellow_loveless.png".to_string(),
                            Some(title.unwrap())
                            );

    Ok(warp::redirect(format!("/{}/catalog", board).parse::<Uri>().unwrap()))
}


// Main server method - using tokio runtime

#[tokio::main]
pub async fn serve<DB: 'static + db::Database+Sync+Send,
                   FR: 'static + fr::FileRack+Sync+Send>(pages: pages::Pages,
                                                         actions: actions::Actions,
                                                         database: DB,
                                                         file_rack: FR,
                                                         ip: [u8; 4], port: u16) {

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
    let catalog = warp::path!(String / "catalog")
                       .and(pages.clone()).and(database.clone())
                       .map(| board: String, p : Pages, db: Arc<Mutex<DB>> | {

        let pages = &mut (*p.lock().unwrap());
        if let Some(board_id) = pages.board_url_to_id(&board) {
            let database = &(*db.lock().unwrap());
            let page_ref = pages::PageRef::Catalog(*board_id);
            let page = pages.get_page(database, &page_ref).unwrap().page_text.to_string();

            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(page)
        }
        else {
            Response::builder().status(404).body("Not Found".to_string())
        }

    });

    // Serve thread pages
    let thread = warp::path!(String / "thread" / u64)
                      .and(pages.clone()).and(database.clone())
                      .map(| board: String, orig_num: u64, p: Pages, db: Arc<Mutex<DB>> | {

        let pages = &mut (*p.lock().unwrap());

        if let Some(board_id) =  pages.board_url_to_id(&board) { 
            let database = &(*db.lock().unwrap());
            let page_ref = pages::PageRef::Thread(*board_id, orig_num);
            let page = pages.get_page(database, &page_ref);

            match page {
                Ok(page) => {
                    Response::builder()
                        .header("Content-Type", "text/html; charset=utf-8")
                        .body(page.page_text.to_string())
                },
                Err(_) => {
                    Response::builder().status(404).body("Not Found".to_string())
                },
            }
        }
        else {
            Response::builder().status(404).body("Not Found".to_string())
        }

    });

    // Serve thread creation page
    let create = warp::path!(String / "create")
                       .and(pages.clone()).and(database.clone())
                       .map(| board: String, p : Pages, db: Arc<Mutex<DB>> | {

        let pages = &mut (*p.lock().unwrap());
        if let Some(board_id) = pages.board_url_to_id(&board) {
            let database = &(*db.lock().unwrap());
            let page_ref = pages::PageRef::Create(*board_id);
            let page = pages.get_page(database, &page_ref).unwrap().page_text.to_string();

            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(page)
        }
        else {
            Response::builder().status(404).body("Not Found".to_string())
        }

    });

    // Serve submit action
    let submit = warp::path!(String / "submit")
                       .and(warp::multipart::form())
                       .and(pages.clone()).and(actions.clone())
                       .and(database.clone()).and(file_rack.clone())
                       .and_then(create_submit);

    // Serve rack files
    let files = warp::path!("files" / String)
                      .and(file_rack.clone())
                      .map(| file_id: String, fr: Arc<Mutex<FR>> | {

        let file_rack = &(*fr.lock().unwrap());
        match file_rack.get_file(&file_id) {
            Ok(file) => Response::builder().body(file),
            Err(err) => Response::builder().status(404).body(Bytes::from("Not Found")),
        }

    });

    // Serve static resources
    let stat = warp::path("static").and(warp::fs::dir("./static"));

    // Bundle routes together and run
    let routes = warp::get().and(stat
                                 .or(files)
                                 .or(catalog)
                                 .or(thread)
                                 .or(create))
                       .or(warp::post().and(submit));
    
    warp::serve(routes).run((ip, port)).await;
}

