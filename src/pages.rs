use crate::site::Post;
use crate::db;
use crate::template;
use crate::util;
use std::collections::HashMap;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum PageRef {
    Catalog(u64),
    Thread(u64),
}

pub struct Page {
    pub page_ref: PageRef,
    pub render_time: u64,
    pub page_text: String,
}

pub struct Pages<'db, DB: db::Database> {
    database: &'db DB,
    pages: HashMap<PageRef, Page>,
    catalog_template: template::Template,
}

impl<'db, DB: db::Database> Pages<'db, DB> {
    pub fn get_page(&self, pr: &PageRef) -> Option<&Page> {
        self.pages.get(pr)
    }
    pub fn render_catalogs(&mut self) -> Result<(), db::DatabaseErr> {
        let boards = self.database.get_boards();
        for board in boards {
            let pr = PageRef::Catalog(board.id);
            let mut values = HashMap::new();
            values.insert(String::from("board_url"), String::from(board.url));
            values.insert(String::from("board_title"), String::from(board.title));

            let mut originals = vec![];
            let cat = self.database.get_catalog(board.id)?;
            for orig in cat.originals {
                values.insert(format!("original.{}.file_url", orig.post_num()), String::from("yellow-loveless.jpg"));
                values.insert(format!("original.{}.replies", orig.post_num()), orig.replies().to_string());
                values.insert(format!("original.{}.img_replies", orig.post_num()), orig.img_replies().to_string());
                values.insert(format!("original.{}.post_title", orig.post_num()), orig.title().unwrap_or("").to_string());
                values.insert(format!("original.{}.post_body", orig.post_num()), orig.body().to_string());

                originals.push(orig.post_num().to_string());
            }
            let mut collections = HashMap::new();
            collections.insert("original".to_string(), originals);
            let render_data = template::Data::new(values, collections);
            let page_text = self.catalog_template.render(&render_data);
            self.pages.insert(pr, Page { page_ref: pr,
                                    render_time: util::timestamp(), page_text });
        }
        Ok(())
    }

    pub fn new(database: &'db DB) -> Result<Pages<'db, DB>, db::DatabaseErr> {
        let mut pages = HashMap::new();
        let cat_tmpl = template::Template::from_file("templates/catalog.html.tmpl").unwrap();
        Ok(Pages {
            database,
            pages,
            catalog_template: cat_tmpl,
        })
    }
}
