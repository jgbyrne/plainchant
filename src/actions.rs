use crate::db;
use crate::fr;
use crate::site;
use crate::site::Post;
use crate::util;
use crate::util::{ErrOrigin, PlainchantErr};
use rand::Rng;
use sha256;
use std::collections::HashMap;
use std::iter;
use std::sync::RwLock;

const TRIPCODE_LEN: usize = 10;

fn compute_tripcode(trip: String) -> String {
    (sha256::digest(trip)[..TRIPCODE_LEN]).to_string()
}

pub struct Actions {
    ban_cache: RwLock<HashMap<String, site::Ban>>,
}

pub enum SubmissionResult {
    Success(u64),
    Banned,
}

impl Actions {
    pub fn new<DB: db::Database>(database: &DB) -> Result<Actions, PlainchantErr> {
        let bans = database.get_bans()?;

        let mut ban_cache = HashMap::<String, site::Ban>::new();

        for ban in bans {
            if !ban_cache.contains_key(&ban.ip)
                || ban_cache[&ban.ip].time_expires < ban.time_expires
            {
                ban_cache.insert(ban.ip.clone(), ban);
            }
        }

        Ok(Actions {
            ban_cache: RwLock::new(ban_cache),
        })
    }

    fn is_banned(&self, ip: &str, cur_time: u64) -> Result<bool, PlainchantErr> {
        let rg = match self.ban_cache.read() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PlainchantErr {
                    origin: ErrOrigin::Actions,
                    msg:    String::from("Failed to read from Ban Cache"),
                })
            },
        };

        match rg.get(ip) {
            Some(ban) => Ok(ban.time_expires > cur_time),
            None => Ok(false),
        }
    }

    pub fn ban_ip<DB: db::Database>(
        &self,
        database: &DB,
        ip: &str,
        ban_length: u64,
    ) -> Result<(), PlainchantErr> {
        let cur_time = util::timestamp();
        let time_expires = cur_time + ban_length;

        let mut wg = match self.ban_cache.write() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(PlainchantErr {
                    origin: ErrOrigin::Actions,
                    msg:    String::from("Failed to write to Ban Cache"),
                })
            },
        };

        let ban = site::Ban {
            id: 0,
            ip: String::from(ip),
            time_expires,
        };

        wg.insert(String::from(ip), ban.clone());

        database.create_ban(ban)
    }

    pub fn upload_file<FR: fr::FileRack>(
        &self,
        file_rack: &FR,
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
        &self,
        database: &DB,
        board_id: u64,
        ip: String,
        body: String,
        poster: Option<String>,
        trip: Option<String>,
        file_id: String,
        file_name: String,
        title: Option<String>,
    ) -> Result<SubmissionResult, util::PlainchantErr> {
        let cur_time = util::timestamp();

        if self.is_banned(&ip, cur_time)? {
            return Ok(SubmissionResult::Banned);
        }

        let feather = match trip {
            None => site::Feather::None,
            Some(t) => site::Feather::Trip(compute_tripcode(t)),
        };

        let original = site::Original {
            board_id,
            post_num: 0,
            time: cur_time,
            ip,
            body,
            poster,
            feather,
            file_id: Some(file_id),
            file_name: Some(file_name),
            title,
            bump_time: cur_time,
            replies: 0,
            img_replies: 0,
            pinned: false,
            archived: false,
        };

        database
            .create_original(original)
            .map(|num| SubmissionResult::Success(num))
    }

    pub fn submit_reply<DB: db::Database>(
        &self,
        database: &DB,
        board_id: u64,
        ip: String,
        body: String,
        poster: Option<String>,
        trip: Option<String>,
        file_id: Option<String>,
        file_name: Option<String>,
        orig_num: u64,
    ) -> Result<SubmissionResult, util::PlainchantErr> {
        let cur_time = util::timestamp();

        if self.is_banned(&ip, cur_time)? {
            return Ok(SubmissionResult::Banned);
        }

        let feather = match trip {
            None => site::Feather::None,
            Some(t) => site::Feather::Trip(compute_tripcode(t)),
        };

        let reply = site::Reply {
            board_id,
            post_num: 0,
            time: cur_time,
            ip,
            body,
            poster,
            feather,
            file_id: file_id.clone(),
            file_name,
            orig_num,
        };

        database
            .create_reply(reply)
            .map(|num| SubmissionResult::Success(num))
    }

    pub fn delete_thread<DB: db::Database, FR: fr::FileRack>(
        &self,
        database: &DB,
        file_rack: &FR,
        board_id: u64,
        post_num: u64,
    ) -> Result<(), util::PlainchantErr> {
        let thread = database.get_thread(board_id, post_num)?;

        database.delete_original(board_id, thread.original.post_num())?;

        if let Some(id) = thread.original.file_id() {
            file_rack.delete_file(id)?;
        }

        for reply in thread.replies {
            if let Some(id) = reply.file_id() {
                file_rack.delete_file(id)?;
            }
        }

        Ok(())
    }

    pub fn enforce_post_cap<DB: db::Database, FR: fr::FileRack>(
        &self,
        database: &DB,
        file_rack: &FR,
        board_id: u64,
    ) -> Result<(), util::PlainchantErr> {
        let board = database.get_board(board_id)?;
        let mut catalog = database.get_catalog(board_id)?;

        let post_cap: usize = board.post_cap.into();

        if catalog.originals.len() > post_cap {
            let excess: Vec<site::Original> = catalog.originals.drain(post_cap..).collect();
            for orig in excess.iter() {
                self.delete_thread(database, file_rack, board_id, orig.post_num)?;
            }
        }
        Ok(())
    }
}
