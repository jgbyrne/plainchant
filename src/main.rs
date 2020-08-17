mod util;
mod site;
mod db;
use db::Database;
mod pages;
mod actions;
mod fsdb;
mod template;
mod server;
use std::collections::HashMap;

fn main() {
    // Load database - this needs to be db::Database - we use the filesystem
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();

    // Load templates from template files 
    let templates = pages::SiteTemplates {
        catalog_tmpl:
            template::Template::from_file("templates/catalog.html.tmpl").unwrap(),
        thread_tmpl:
            template::Template::from_file("templates/thread.html.tmpl").unwrap(),
        create_tmpl:
            template::Template::from_file("templates/create.html.tmpl").unwrap(),
    };

    // Create structs for pages and actions
    let mut pages = pages::Pages::new(&db, templates, 1).unwrap();
    let mut actions = actions::Actions::new();

    // Serve the site using the pages, actions, and database
    server::serve(pages, actions, db, [0, 0, 0, 0], 8080);
}
