mod site;
mod util;

mod db;

mod fr;

mod actions;
mod fsdb;
mod fsfr;
mod pages;
mod server;
mod template;

fn main() {
    // Load database - this needs to be db::Database - we use the filesystem
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();

    // Load file rack - this needs to be fr::FileRack - we use the filesystem
    let fr = fsfr::FSFileRack::from_dir("./fstest/rack").unwrap();

    // Load templates from template files
    let templates = pages::SiteTemplates {
        catalog_tmpl: template::Template::from_file("templates/catalog.html.tmpl").unwrap(),
        thread_tmpl: template::Template::from_file("templates/thread.html.tmpl").unwrap(),
        create_tmpl: template::Template::from_file("templates/create.html.tmpl").unwrap(),
    };

    // Create structs for pages and actions
    let pages = pages::Pages::new(&db, templates, 1).unwrap();
    let actions = actions::Actions::new();

    // Serve the site using the pages, actions, and database
    server::serve(pages, actions, db, fr, [0, 0, 0, 0], 8088);
}
