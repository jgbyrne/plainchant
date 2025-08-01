use crate::db;
use crate::format;
use crate::site;
use crate::site::Post;
use crate::template;
use crate::util;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};

// Maximum number of post replies to process in a given post
// Prevents maliciously tagging everyone in a thread
const MAX_INLINE_REPLIES: usize = 24;

pub struct StaticPages {
    pub error_tmpl:   template::Template,
    pub message_tmpl: template::Template,
}

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

fn clone_option_string_or_empty(o_str: &Option<String>) -> String {
    o_str
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn compute_fwd_links(thread: &db::Thread, posts: &HashSet<u64>) -> HashMap<u64, Vec<u64>> {
    lazy_static! {
        // Capture replies of form >>390290
        static ref REPLY: Regex = Regex::new(r"(>>)([\d]+)(?:$|\W)").unwrap();
    }

    let mut links = HashMap::new();

    for reply in &thread.replies {
        let referenced_posts = REPLY.captures_iter(&reply.body).take(MAX_INLINE_REPLIES);
        for ref_post in referenced_posts {
            let ref_post_num = ref_post.get(2).unwrap();

            let ref_post_num = match &ref_post_num.as_str().parse::<u64>() {
                Ok(n) => *n,
                Err(_) => continue,
            };

            // Do not include posts from outside the thread
            if !posts.contains(&ref_post_num) {
                continue;
            }

            // Do not forward link to one post multiple times
            let fwd_list = links.entry(ref_post_num).or_insert(vec![]);
            if let Some(last) = fwd_list.last() {
                if *last == reply.post_num() {
                    continue;
                }
            }

            // Only forward link to newer posts
            if ref_post_num >= reply.post_num() {
                continue;
            }

            fwd_list.push(reply.post_num());
        }
    }

    links
}

impl Pages {
    pub fn render<DB: db::Database>(
        &self,
        database: &DB,
        pr: &PageRef,
    ) -> Result<Page, util::PlainchantErr> {
        match pr {
            PageRef::Homepage => {
                let mut render_data = template::Data::full();

                render_data.insert_value("site_name", self.site.name.clone());
                render_data.insert_value("site_description", self.site.description.clone());
                render_data.insert_value(
                    "site_contact",
                    clone_option_string_or_empty(&self.site.contact),
                );

                let mut board_ids = vec![];
                let boards = database.get_boards()?;

                for board in boards {
                    render_data.insert_collection_value(
                        "board",
                        board.id,
                        "url",
                        board.url.to_string(),
                    );
                    render_data.insert_collection_value(
                        "board",
                        board.id,
                        "title",
                        board.title.to_string(),
                    );
                    board_ids.push(board.id.to_string());
                }

                render_data.add_collection("board", board_ids);

                let page_text = self.templates.homepage_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
            PageRef::Catalog(board_id) => {
                let board = database.get_board(*board_id)?;

                let mut render_data = template::Data::full();

                render_data.insert_value("site_name", self.site.name.clone());
                render_data.insert_value(
                    "site_contact",
                    clone_option_string_or_empty(&self.site.contact),
                );

                render_data.insert_value("board_url", board.url);
                render_data.insert_value("board_title", board.title);

                let mut originals = vec![];
                let cat = database.get_catalog(board.id)?;
                for orig in cat.originals {
                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "file_url",
                        format!("/thumbnails/{}", orig.file_id().unwrap_or("")),
                    );

                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "replies",
                        orig.replies().to_string(),
                    );

                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "img_replies",
                        orig.img_replies().to_string(),
                    );

                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "post_num",
                        orig.post_num().to_string(),
                    );

                    let mut cat_title = orig.title().unwrap_or("").to_string();
                    if let Some((i, _)) = cat_title.char_indices().nth(64) {
                        cat_title.truncate(i);
                    }

                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "post_title",
                        format::html_escape_and_trim(&cat_title),
                    );

                    let mut cat_desc = orig.body().to_string();
                    if let Some((i, _)) = cat_desc.char_indices().nth(100) {
                        cat_desc.truncate(i);
                    }

                    render_data.insert_collection_value(
                        "original",
                        orig.post_num(),
                        "post_body",
                        format::html_escape_and_trim(&cat_desc),
                    );

                    originals.push(orig.post_num().to_string());
                }

                render_data.add_collection("original", originals);

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

                let mut render_data = template::Data::full();

                // The set of post IDs in the current thread is used
                // by the annotate_post function to decide how whether
                // to use an anchor link or a direct link
                let mut posts = HashMap::new();

                posts.insert(
                    thread.original.post_num(),
                    format::html_escape_and_trim(thread.original.body()),
                );

                for reply in &thread.replies {
                    posts.insert(reply.post_num(), format::html_escape_and_trim(reply.body()));
                }

                let posts = posts;

                let fwd_links = compute_fwd_links(&thread, &posts.keys().cloned().collect());

                render_data.insert_value("site_name", self.site.name.clone());
                render_data.insert_value(
                    "site_contact",
                    clone_option_string_or_empty(&self.site.contact),
                );
                render_data.insert_value("site_url", clone_option_string_or_empty(&self.site.url));

                render_data.insert_value("board_url", board.url);
                render_data.insert_value("board_title", board.title);

                render_data.insert_value("replies", thread.original.replies().to_string());

                render_data.insert_value("img_replies", thread.original.img_replies().to_string());

                render_data.insert_value(
                    "orig_file_url",
                    format!("/files/{}", thread.original.file_id().unwrap_or("")),
                );

                render_data.insert_value(
                    "orig_thumbnail_url",
                    format!("/thumbnails/{}", thread.original.file_id().unwrap_or("")),
                );

                let title = thread
                    .original
                    .title()
                    .map(format::html_escape_and_trim);

                render_data.set_flag("orig_has_title", title.is_some());

                render_data.insert_value("orig_title", title.unwrap_or(String::from("")));

                render_data.insert_value(
                    "orig_poster",
                    thread
                        .original
                        .poster()
                        .map(format::html_escape_and_trim)
                        .unwrap_or(String::from("Anonymous")),
                );

                render_data
                    .insert_value("orig_time", format::humanise_time(thread.original.time()));

                render_data.insert_value(
                    "orig_timestamp",
                    format::utc_timestamp(thread.original.time()),
                );

                render_data.insert_value("orig_post_num", thread.original.post_num().to_string());

                render_data.insert_value(
                    "orig_feather",
                    format::display_feather(thread.original.feather()),
                );

                render_data.insert_value(
                    "orig_fwd_links",
                    format::annotate_fwd_links(
                        thread.original.post_num(),
                        &posts,
                        fwd_links
                            .get(&thread.original.post_num())
                            .unwrap_or(&vec![]),
                    ),
                );

                render_data.insert_value(
                    "orig_post_body",
                    format::annotate_post(
                        &posts[&thread.original.post_num()],
                        &posts,
                        thread.original.post_num(),
                        false,
                    ),
                );

                let mut replies = vec![];
                for reply in thread.replies {
                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "file_url",
                        format!("/files/{}", reply.file_id().unwrap_or("")),
                    );

                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "thumbnail_url",
                        format!("/thumbnails/{}", reply.file_id().unwrap_or("")),
                    );

                    render_data.set_collection_flag(
                        "reply",
                        reply.post_num(),
                        "has_image",
                        reply.file_id().is_some(),
                    );

                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "poster",
                        reply
                            .poster()
                            .map(format::html_escape_and_trim)
                            .unwrap_or(String::from("Anonymous")),
                    );

                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "time",
                        format::humanise_time(reply.time()),
                    );

                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "timestamp",
                        format::utc_timestamp(reply.time()),
                    );
                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "feather",
                        format::display_feather(reply.feather()),
                    );
                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "post_num",
                        reply.post_num().to_string(),
                    );
                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "fwd_links",
                        format::annotate_fwd_links(
                            thread.original.post_num(),
                            &posts,
                            fwd_links.get(&reply.post_num()).unwrap_or(&vec![]),
                        ),
                    );

                    render_data.insert_collection_value(
                        "reply",
                        reply.post_num(),
                        "post_body",
                        format::annotate_post(
                            &posts[&reply.post_num()],
                            &posts,
                            thread.original.post_num(),
                            false,
                        ),
                    );

                    replies.push(reply.post_num());
                }

                replies.sort();

                render_data
                    .add_collection("reply", replies.iter().map(|r| r.to_string()).collect());

                let page_text = self.templates.thread_tmpl.render(&render_data);
                Ok(Page {
                    page_ref: *pr,
                    render_time: util::timestamp(),
                    page_text,
                })
            },
            PageRef::Create(board_id) => {
                let board = database.get_board(*board_id)?;

                let mut render_data = template::Data::simple();

                render_data.insert_value("site_name", self.site.name.clone());
                render_data.insert_value(
                    "site_contact",
                    clone_option_string_or_empty(&self.site.contact),
                );

                render_data.insert_value("board_url", board.url);
                render_data.insert_value("board_title", board.title);

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
