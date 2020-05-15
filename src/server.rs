use crate::db;
use crate::pages;
use crate::actions;
use crate::util;
use std::convert::Infallible;
use warp::{http::Uri, Filter};
use warp::multipart;
use warp::reply::Reply;
use warp::http::Response;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use futures::StreamExt;
use bytes::{BytesMut, BufMut};

type Pages = Arc<Mutex<pages::Pages>>;
type Actions = Arc<Mutex<actions::Actions>>;

async fn part_string(part: multipart::Part, buf_size: usize) -> Option<String> {
    let mut chunks = part.stream();
    let mut buffer = BytesMut::with_capacity(buf_size);
    while let Some(Ok(buf)) = chunks.next().await {
        buffer.put(buf); 
    }

    match std::str::from_utf8(&buffer[..]) {
        Ok(s) => Some(String::from(s)),
        Err(_) => None,
    }
}

async fn create_submit<DB: 'static + db::Database+Sync+Send>
                      (board: String, mut data: multipart::FormData,
                       p: Pages, a: Actions, db: Arc<Mutex<DB>>) -> Result<impl warp::Reply, Infallible> {
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
 
            },
            _ => {},
        }
    }
    let mut actions = a.lock().unwrap();
    actions.submit_original(&mut *db.lock().unwrap(),
                            board_id, "0.0.0.0".to_string(),
                            body.unwrap(),
                            Some(name.unwrap()),
                            "yellow_loveless".to_string(),
                            "yellow_loveless.png".to_string(),
                            Some(title.unwrap())
                            );
    Ok(warp::redirect(format!("/{}/catalog", board).parse::<Uri>().unwrap()))
}

#[tokio::main]
pub async fn serve<DB: 'static + db::Database+Sync+Send>(pages: pages::Pages,
                                                         actions: actions::Actions,
                                                         database: DB,
                                                         ip: [u8; 4], port: u16) {

    let pages = Arc::new(Mutex::new(pages));
    let pages = warp::any().map(move || pages.clone());

    let actions = Arc::new(Mutex::new(actions));
    let actions = warp::any().map(move || actions.clone());

    let database = Arc::new(Mutex::new(database));
    let database = warp::any().map(move || database.clone());

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

    let submit = warp::path!(String / "submit")
                       .and(warp::multipart::form())
                       .and(pages.clone()).and(actions.clone()).and(database.clone())
                       .and_then(create_submit);

    let stat = warp::path("static").and(warp::fs::dir("./static"));

    let routes = warp::get().and(stat
                                 .or(catalog)
                                 .or(thread)
                                 .or(create))
                       .or(warp::post().and(submit));
    
    warp::serve(routes).run((ip, port)).await;
}

