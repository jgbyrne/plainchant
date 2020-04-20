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

    let catalog = warp::path!(String / "catalog").and(pages.clone()).and(database.clone())
                                                 .map(| board: String, p : Arc<Mutex<pages::Pages>>, db: Arc<Mutex<DB>> | {
        
        let pages = &mut (*p.lock().unwrap());
        if let Some(ref board_id) = pages.board_url_to_id(&board) {
            let database = &(*db.lock().unwrap());
            let page = pages.get_page(database, &pages::PageRef::Catalog(1234)).unwrap().page_text.to_string();
            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(page)
        }
        else {
            Response::builder()
                .status(404)
                .body("Not Found".to_string())
        }
    });

    let stat = warp::path("static").and(warp::fs::dir("./static"));

    let routes = warp::get().and(stat.or(catalog));
    warp::serve(routes).run((ip, port)).await;
}

