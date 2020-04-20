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

pub struct Pages {
    pages: HashMap<PageRef, Page>,
    templates: SiteTemplates,
    render_freq: u64,
    board_urls: HashMap<String, u64>,
}

impl Pages {
    pub fn render<DB: db::Database>(&mut self, database: &DB, pr: &PageRef) -> Result<&Page, util::PlainchantErr> {
        match pr {
            PageRef::Catalog(board_id) => { 
                match database.get_board(*board_id) {
                    Ok(board) => {
                        let mut values = HashMap::new();
                        values.insert(String::from("board_url"), String::from(board.url));
                        values.insert(String::from("board_title"), String::from(board.title));

                        let mut originals = vec![];
                        let cat = database.get_catalog(board.id)?;
                        for orig in cat.originals {

                            values.insert(format!("original.{}.file_url", orig.post_num()),
                                          String::from("/static/yellow-loveless.jpg"));

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
                        self.pages.insert(*pr, page);
                        Ok(self.pages.get(pr).unwrap())
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

    pub fn page_exists<DB: db::Database>(&self, database: &DB, pr: &PageRef) -> bool {
        match pr {
            PageRef::Catalog(board_id) => {
                match database.get_board(*board_id) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            },
            PageRef::Thread(orig_num) => {
                unimplemented!();
            },
        }
    }

    pub fn get_page<DB: db::Database>(&mut self, database: &DB, pr: &PageRef) -> Result<&Page, util::PlainchantErr> {
        match self.pages.get(pr) {
            Some(page) =>  {
               let now = util::timestamp();
               if now - page.render_time > self.render_freq {
                   return self.render(database, pr);
               }
            },
            None => {
                if self.page_exists(database, pr) {
                    return self.render(database, pr);
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

    pub fn board_url_to_id(&self, url: &str) -> Option<&u64> {
        self.board_urls.get(url)
    }

    pub fn new<DB: db::Database>(database: &DB, templates: SiteTemplates, render_freq: u64) -> Result<Pages, util::PlainchantErr> {
        let mut pages = HashMap::new();
        
        let mut board_urls = HashMap::new();
        for board in database.get_boards() {
            board_urls.insert(board.url.clone(), board.id);
        }

        Ok(Pages {
            pages,
            templates,
            render_freq,
            board_urls,
        })
    }
}
