mod util;
mod site;
mod db;
use db::Database;
mod pages;
mod fsdb;
mod template;
use std::collections::HashMap;

fn main() {
    /*
    println!("Hello, world!");
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();
    println!("{:?}", db.get_boards());
    println!("{:?}", db.get_board(1234));
    println!("{:?}", db.get_board(1244));
    println!("{:#?}\n", db.get_original(1234, 5678));
    println!("{:#?}", db.get_thread(1234, 5678));
    println!("{:#?}", db.get_reply(1234, 5681));
    println!("{:#?}", db.get_post(1234, 5678).unwrap().ip());
    println!("{:#?}", db.get_catalog(1234));
    let mut values = HashMap::new();
    values.insert(String::from("board_url"), String::from("mu"));
    values.insert(String::from("board_title"), String::from("Music"));
    values.insert(String::from("original.1.file_url"), String::from("./yellow-loveless.jpg"));
    values.insert(String::from("original.1.replies"), String::from("0"));
    values.insert(String::from("original.1.img_replies"), String::from("0"));
    values.insert(String::from("original.1.post_title"), String::from("Test Title"));
    values.insert(String::from("original.1.post_body"), String::from("Test Body"));

    let mut collections = HashMap::new();
    collections.insert(String::from("original"), vec![String::from("1")]);

    let tmpl = template::Template::from_file("./templates/catalog.html.tmpl").unwrap();
    let data = template::Data::new(values, collections); 
    println!("{}", tmpl.render(&data));
    */
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();

    let templates = pages::SiteTemplates {
        catalog_tmpl: template::Template::from_file("templates/catalog.html.tmpl").unwrap(),
    };
    let mut pages = pages::Pages::new(&db, templates, 2).unwrap();
    let page = pages.get_page(&pages::PageRef::Catalog(1234)).unwrap();
    println!("{}", page.page_text);
}
