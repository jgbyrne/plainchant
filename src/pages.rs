use crate::db;
use crate::format;
use crate::site;
use crate::site::Post;
use crate::template;
use crate::util;
use std::collections::{HashMap, HashSet};

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum PageRef {
    Homepage,
    Catalog(u64),
    Thread(u64, u64),
    Create(u64),
}

pub struct Page {
    pub page_ref:    PageRef,
    pub render_time: u64,
    pub page_text:   String,
}

pub struct SiteTemplates {
    pub homepage_tmpl: template::Template,
    pub catalog_tmpl:  template::Template,
    pub thread_tmpl:   template::Template,
    pub create_tmpl:   template::Template,
}

pub struct Pages {
    site:        site::Site,
    pages:       HashMap<PageRef, Page>,
    templates:   SiteTemplates,
    render_freq: u64,
}

impl Pages {
    pub fn render<DB: db::Database>(
        &self,
        database: &DB,
        pr: &PageRef,
    ) -> Result<Page, util::PlainchantErr> {
        match pr {
            PageRef::Homepage => {
                let mut values = HashMap::new();

                values.insert(String::from("site_name"), self.site.name.clone());
                values.insert(
                    String::from("site_description"),
                    self.site.description.clone(),
                );

                let mut board_ids = vec![];
                let boards = database.get_boards()?;

                for board in boards {
                    values.insert(format!("board.{}.url", board.id), String::from(board.url));
                    values.insert(
                        format!("board.{}.title", board.id),
                        String::from(board.title),
                    );

                    board_ids.push(board.id.to_string());
                }

                let mut collections = HashMap::new();
                collections.insert("board".to_string(), board_ids);

                let render_data = template::Data::new(values, HashMap::new(), collections);

                let page_text = self.templates.homepage_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
            PageRef::Catalog(board_id) => {
                let board = database.get_board(*board_id)?;
                let mut values = HashMap::new();

                values.insert(String::from("site_name"), self.site.name.clone());

                values.insert(String::from("board_url"), board.url);
                values.insert(String::from("board_title"), board.title);

                let mut originals = vec![];
                let cat = database.get_catalog(board.id)?;
                for orig in cat.originals {
                    values.insert(
                        format!("original.{}.file_url", orig.post_num()),
                        format!("/thumbnails/{}", orig.file_id().unwrap_or("")),
                    );

                    values.insert(
                        format!("original.{}.replies", orig.post_num()),
                        orig.replies().to_string(),
                    );

                    values.insert(
                        format!("original.{}.img_replies", orig.post_num()),
                        orig.img_replies().to_string(),
                    );

                    values.insert(
                        format!("original.{}.post_num", orig.post_num()),
                        orig.post_num().to_string(),
                    );

                    let mut cat_title = orig.title().unwrap_or("").to_string();
                    if let Some((i, _)) = cat_title.char_indices().nth(64) {
                        cat_title.truncate(i);
                    }

                    values.insert(
                        format!("original.{}.post_title", orig.post_num()),
                        format::html_escape(&cat_title),
                    );

                    let mut cat_desc = orig.body().to_string();
                    if let Some((i, _)) = cat_desc.char_indices().nth(128) {
                        cat_desc.truncate(i);
                    }
                    values.insert(
                        format!("original.{}.post_body", orig.post_num()),
                        format::html_escape(&cat_desc),
                    );

                    originals.push(orig.post_num().to_string());
                }
                let mut collections = HashMap::new();
                collections.insert("original".to_string(), originals);
                let render_data = template::Data::new(values, HashMap::new(), collections);
                let page_text = self.templates.catalog_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
            PageRef::Thread(board_id, orig_num) => {
                let board = database.get_board(*board_id)?;
                let thread = database.get_thread(*board_id, *orig_num)?;

                // The set of post IDs in the current thread is used
                // by the annotate_post function to decide how whether
                // to use an anchor link or a direct link
                let mut posts = HashSet::new();
                posts.insert(thread.original.post_num());
                posts.extend(thread.replies.iter().map(|r| r.post_num()));
                let posts = posts;

                let mut values = HashMap::new();
                let mut flags = HashMap::new();

                values.insert(String::from("site_name"), self.site.name.clone());

                values.insert(String::from("board_url"), board.url);
                values.insert(String::from("board_title"), board.title);

                values.insert(
                    String::from("replies"),
                    thread.original.replies().to_string(),
                );
                values.insert(
                    String::from("img_replies"),
                    thread.original.img_replies().to_string(),
                );

                values.insert(
                    String::from("orig_file_url"),
                    format!("/files/{}", thread.original.file_id().unwrap_or("")),
                );
                values.insert(
                    String::from("orig_thumbnail_url"),
                    format!("/thumbnails/{}", thread.original.file_id().unwrap_or("")),
                );
                values.insert(
                    String::from("orig_title"),
                    thread.original.title()
                        .map(|t| format::html_escape(t))
                        .unwrap_or(String::from("")),
                );
                values.insert(
                    String::from("orig_poster"),
                    thread.original.poster()
                        .map(|p| format::html_escape(p))
                        .unwrap_or(String::from("")),
                );
                values.insert(
                    String::from("orig_time"),
                    format::humanise_time(thread.original.time()),
                );
                values.insert(
                    String::from("orig_timestamp"),
                    format::utc_timestamp(thread.original.time()),
                );
                values.insert(
                    String::from("orig_post_num"),
                    thread.original.post_num().to_string(),
                );
                values.insert(
                    String::from("orig_feather"),
                    format::display_feather(thread.original.feather()),
                );
                values.insert(
                    String::from("orig_post_body"),
                    format::annotate_post(
                        &format::html_escape(thread.original.body()),
                        &posts
                    ),
                );

                let mut replies = vec![];
                for reply in thread.replies {
                    values.insert(
                        format!("reply.{}.file_url", reply.post_num()),
                        format!("/files/{}", reply.file_id().unwrap_or("")),
                    );

                    values.insert(
                        format!("reply.{}.thumbnail_url", reply.post_num()),
                        format!("/thumbnails/{}", reply.file_id().unwrap_or("")),
                    );

                    flags.insert(
                        format!("reply.{}.has_image", reply.post_num()),
                        reply.file_id().is_some(),
                    );

                    values.insert(
                        format!("reply.{}.poster", reply.post_num()),
                        reply.poster()
                            .map(|p| format::html_escape(p))
                            .unwrap_or(String::from("Anonymous")),
                    );

                    values.insert(
                        format!("reply.{}.time", reply.post_num()),
                        format::humanise_time(reply.time()),
                    );

                    values.insert(
                        format!("reply.{}.timestamp", reply.post_num()),
                        format::utc_timestamp(reply.time()),
                    );
                    values.insert(
                        format!("reply.{}.feather", reply.post_num()),
                        format::display_feather(reply.feather()),
                    );
                    values.insert(
                        format!("reply.{}.post_num", reply.post_num()),
                        reply.post_num().to_string(),
                    );

                    values.insert(
                        format!("reply.{}.post_body", reply.post_num()),
                        format::annotate_post(
                            &format::html_escape(reply.body()),
                            &posts
                        ),
                    );

                    replies.push(reply.post_num().to_string());
                }

                replies.sort();

                let mut collections = HashMap::new();
                collections.insert("reply".to_string(), replies);

                let render_data = template::Data::new(values, flags, collections);
                let page_text = self.templates.thread_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
            PageRef::Create(board_id) => {
                let board = database.get_board(*board_id)?;
                let mut values = HashMap::new();

                values.insert(String::from("site_name"), self.site.name.clone());

                values.insert(String::from("board_url"), board.url);
                values.insert(String::from("board_title"), board.title);

                let collections = HashMap::new();
                let render_data = template::Data::new(values, HashMap::new(), collections);
                let page_text = self.templates.create_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
        }
    }

    pub fn update(&mut self, pr: &PageRef, page: Page) -> &Page {
        self.pages.insert(*pr, page);
        self.pages.get(pr).unwrap()
    }

    pub fn page_exists<DB: db::Database>(&self, database: &DB, pr: &PageRef) -> bool {
        match pr {
            PageRef::Homepage => true,
            PageRef::Catalog(board_id) => database.get_board(*board_id).is_ok(),
            PageRef::Thread(board_id, orig_num) => {
                database.get_thread(*board_id, *orig_num).is_ok()
            },
            PageRef::Create(board_id) => database.get_board(*board_id).is_ok(),
        }
    }

    pub fn get_page<DB: db::Database>(
        &self,
        database: &DB,
        pr: &PageRef,
    ) -> Result<Option<&Page>, util::PlainchantErr> {
        match self.pages.get(pr) {
            Some(page) => {
                let now = util::timestamp();
                if now - page.render_time > self.render_freq {
                    return Ok(None);
                }
            },
            None => {
                if self.page_exists(database, pr) {
                    return Ok(None);
                } else {
                    return Err(util::PlainchantErr {
                        origin: util::ErrOrigin::Web(404),
                        msg:    "No such page".to_string(),
                    });
                }
            },
        }
        // Borrow checker makes us get again >:-(
        Ok(Some(self.pages.get(pr).unwrap()))
    }

    pub fn new(
        site: site::Site,
        templates: SiteTemplates,
        render_freq: u64,
    ) -> Result<Pages, util::PlainchantErr> {
        let pages = HashMap::new();

        Ok(Pages {
            site,
            pages,
            templates,
            render_freq,
        })
    }
}
