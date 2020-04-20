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

pub struct SiteTemplates {
    pub catalog_tmpl : template::Template,
//    pub thread_tmpl : template::Template,
}

pub struct Pages<'db, DB: db::Database> {
    database: &'db DB,
    pages: HashMap<PageRef, Page>,
    templates: SiteTemplates,
    render_freq: u64,
}

impl<'db, DB: db::Database> Pages<'db, DB> {
    pub fn render(&mut self, pr: &PageRef) -> Result<&Page, util::PlainchantErr> {
        match pr {
            PageRef::Catalog(board_id) => { 
                match self.database.get_board(*board_id) {
                    Ok(board) => {
                        let mut values = HashMap::new();
                        values.insert(String::from("board_url"), String::from(board.url));
                        values.insert(String::from("board_title"), String::from(board.title));

                        let mut originals = vec![];
                        let cat = self.database.get_catalog(board.id)?;
                        for orig in cat.originals {

                            values.insert(format!("original.{}.file_url", orig.post_num()),
                                          String::from("yellow-loveless.jpg"));

                            values.insert(format!("original.{}.replies", orig.post_num()),
                                          orig.replies().to_string());

                            values.insert(format!("original.{}.img_replies", orig.post_num()),
                                          orig.img_replies().to_string());

                            values.insert(format!("original.{}.post_title", orig.post_num()),
                                          orig.title().unwrap_or("").to_string());

                            values.insert(format!("original.{}.post_body", orig.post_num()),
                                          orig.body().to_string());

                            originals.push(orig.post_num().to_string());
                        }
                        let mut collections = HashMap::new();
                        collections.insert("original".to_string(), originals);
                        let render_data = template::Data::new(values, collections);
                        let page_text = self.templates.catalog_tmpl.render(&render_data);
                        let page = Page { page_ref: *pr,
                                          render_time: util::timestamp(), page_text };
                        let page_entry = self.pages.entry(*pr).or_insert(page);
                        Ok(page_entry)
                    },
                    Err(e) => {
                        Err(e)
                    },
                }


           
            },
            PageRef::Thread(orig_num) => {
                unimplemented!();
            },
        }
    }

    pub fn page_exists(&self, pr: &PageRef) -> bool {
        match pr {
            PageRef::Catalog(board_id) => {
                match self.database.get_board(*board_id) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            },
            PageRef::Thread(orig_num) => {
                unimplemented!();
            },
        }
    }

    pub fn get_page(&mut self, pr: &PageRef) -> Result<&Page, util::PlainchantErr> {
        match self.pages.get(pr) {
            Some(page) =>  {
               let now = util::timestamp();
               if now - page.render_time > self.render_freq {
                   return self.render(pr);
               }
            },
            None => {
                if self.page_exists(pr) {
                    return self.render(pr);
                }
                else {
                    return Err(util::PlainchantErr {
                        origin: util::ErrOrigin::Web(404),
                        msg: "No such page".to_string(),
                    });
                }
            },
        }
        // Borrow checker makes us get again >:-(
        Ok(self.pages.get(pr).unwrap())
    }
    
    pub fn new(database: &'db DB, templates: SiteTemplates, render_freq: u64) -> Result<Pages<'db, DB>, util::PlainchantErr> {
        let mut pages = HashMap::new();
        Ok(Pages {
            database,
            pages,
            templates,
            render_freq,
        })
    }
}
