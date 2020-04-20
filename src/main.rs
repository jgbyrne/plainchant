mod util;
mod site;
mod db;
use db::Database;
mod pages;
mod fsdb;
mod template;
mod server;
use std::collections::HashMap;

fn main() {
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();

    let templates = pages::SiteTemplates {
        catalog_tmpl:
            template::Template::from_file("templates/catalog.html.tmpl").unwrap(),
        thread_tmpl:
            template::Template::from_file("templates/thread.html.tmpl").unwrap(),
    };

    let mut pages = pages::Pages::new(&db, templates, 1).unwrap();
    server::serve(pages, db, [127, 0, 0, 1], 3030);
}
