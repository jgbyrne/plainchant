use crate::db;
use crate::fr;
use crate::site;
use crate::site::Post;
use crate::util;
use rand::Rng;
use std::iter;

pub struct Actions {}

impl Actions {
    pub fn new() -> Actions {
        Actions {}
    }

    pub fn upload_file<FR: fr::FileRack>(
        &mut self,
        file_rack: &mut FR,
        file: bytes::Bytes,
    ) -> Result<String, util::PlainchantErr> {
        let mut rng = rand::thread_rng();
        let file_id: String = iter::repeat(())
            .map(|()| rng.sample(rand::distributions::Alphanumeric) as char)
            .take(12)
            .collect();

        file_rack.store_file(&file_id, file)?;
        Ok(file_id)
    }

    pub fn submit_original<DB: db::Database>(
        &mut self,
        database: &mut DB,
        board_id: u64,
        ip: String,
        body: String,
        poster: Option<String>,
        file_id: String,
        file_name: String,
        title: Option<String>,
    ) -> Result<u64, util::PlainchantErr> {
        let cur_time = util::timestamp();
        let original = site::Original {
            board_id,
            post_num: 0,
            time: cur_time,
            ip,
            body,
            poster,
            feather: site::Feather::None,
            file_id: Some(file_id),
            file_name: Some(file_name),
            title,
            bump_time: cur_time,
            replies: 0,
            img_replies: 0,
            pinned: false,
            archived: false,
        };
        database.create_original(original)
    }

    pub fn enforce_post_cap<DB: db::Database, FR: fr::FileRack>(
        &mut self,
        database: &mut DB,
        file_rack: &mut FR,
        board_id: u64,
    ) -> Result<(), util::PlainchantErr> {
        let board = database.get_board(board_id)?;
        let mut catalog = database.get_catalog(board_id)?;

        let post_cap: usize = board.post_cap.into();

        if catalog.originals.len() > post_cap {
            let excess: Vec<site::Original> = catalog.originals.drain(post_cap..).collect();
            for orig in excess.iter() {
                database.delete_original(board_id, orig.post_num())?;
                if let Some(id) = orig.file_id() {
                    file_rack.delete_file(id)?;
                }
            }
        }
        Ok(())
    }

    pub fn submit_reply<DB: db::Database>(
        &mut self,
        database: &mut DB,
        board_id: u64,
        ip: String,
        body: String,
        poster: Option<String>,
        file_id: Option<String>,
        file_name: Option<String>,
        orig_num: u64,
    ) -> Result<u64, util::PlainchantErr> {
        let board = database.get_board(board_id)?;
        //let mut orig = database.get_original(board_id, orig_num)?;

        let cur_time = util::timestamp();
        let reply = site::Reply {
            board_id,
            post_num: 0,
            time: cur_time,
            ip,
            body,
            poster,
            feather: site::Feather::None,
            file_id: file_id.clone(),
            file_name,
            orig_num,
        };
        let post_id = database.create_reply(reply)?;
        //let new_reply_count = orig.replies() + 1;

        //if new_reply_count <= board.bump_limit {
        //    orig.set_bump_time(cur_time);
        //}

        //orig.set_replies(new_reply_count);
        //if file_id.is_some() {
        //    orig.set_img_replies(orig.img_replies() + 1);
        //}

        //database.update_original(orig)?;

        Ok(post_id)
    }
}
