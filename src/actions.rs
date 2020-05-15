use crate::util;
use crate::site::Post;
use crate::site;
use crate::db;

pub struct Actions {
}

impl Actions {

    pub fn new() -> Actions {
        Actions { }
    }

    pub fn submit_original<DB: db::Database>(&mut self, database: &mut DB,
                           board_id: u64, ip: String, body: String,
                           poster: Option<String>, file_id: String,
                           file_name: String, title: Option<String>) {

        let cur_time = util::timestamp();
        let original = site::Original::new(board_id,
                                           0, // post_num
                                           cur_time,
                                           ip,
                                           body,
                                           poster,
                                           Some(file_id),
                                           Some(file_name),
                                           title,
                                           cur_time,
                                           0,
                                           0);
        database.create_original(original);
    }

}
