use crate::db;
use crate::pages;
use crate::util;
use warp::{Filter};
use warp::http::Response;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[tokio::main]
pub async fn serve<DB: 'static + db::Database+Sync+Send>(pages: pages::Pages, database: DB,
                                                         ip: [u8; 4], port: u16) {

    let pages = Arc::new(Mutex::new(pages));
    let pages = warp::any().map(move || pages.clone());

    let database = Arc::new(Mutex::new(database));
    let database = warp::any().map(move || database.clone());

    let catalog = warp::path!(String / "catalog")
                       .and(pages.clone()).and(database.clone())
                       .map(| board: String, p : Arc<Mutex<pages::Pages>>, db: Arc<Mutex<DB>> | {
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
                      .map(| board: String, orig_num: u64, p: Arc<Mutex<pages::Pages>>, db: Arc<Mutex<DB>> | {
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

    let stat = warp::path("static").and(warp::fs::dir("./static"));

    let routes = warp::get().and(stat.or(catalog).or(thread));
    warp::serve(routes).run((ip, port)).await;
}

