use crate::util;
use crate::site::Post;
use crate::site;
use crate::db;
use crate::fr;
use bytes::Bytes;
use rand;
use rand::Rng;
use std::iter;

pub struct Actions {
}

impl Actions {

    pub fn new() -> Actions {
        Actions { }
    }

    pub fn upload_file<FR: fr::FileRack>(&mut self, file_rack: &mut FR,
                      file: bytes::Bytes)
                      -> Result<String, util::PlainchantErr> {

        let mut rng = rand::thread_rng();
        let file_id: String = iter::repeat(())
                .map(|()| rng.sample(rand::distributions::Alphanumeric))
                .take(12)
                .collect();
        
        file_rack.store_file(&file_id, file)?;
        Ok(file_id)
    }

    pub fn submit_original<DB: db::Database>(&mut self, database: &mut DB,
                           board_id: u64, ip: String, body: String,
                           poster: Option<String>, file_id: String,
                           file_name: String, title: Option<String>)
                           -> Result<u64, util::PlainchantErr> {

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
        database.create_original(original)
    }

    pub fn submit_reply<DB: db::Database>(&mut self, database: &mut DB,
                                          board_id: u64, ip: String, body: String,
                                          poster: Option<String>, file_id: Option<String>,
                                          file_name: Option<String>, orig_num: u64)
                                          -> Result<u64, util::PlainchantErr> {

        let cur_time = util::timestamp();
        let reply    = site::Reply::new(board_id,
                                        0, // post_num
                                        cur_time,
                                        ip,
                                        body,
                                        poster,
                                        file_id,
                                        file_name,
                                        orig_num);
        database.create_reply(reply)
    }

}
