use crate::db;
use crate::fr;
use crate::site;
use crate::site::Post;
use crate::util;
use crate::util::{unwrap_or_return, ErrOrigin, PlainchantErr};
use rand::Rng;
use sha256;
use std::collections::HashMap;
use std::iter;
use std::sync::RwLock;

const TRIPCODE_LEN: usize = 10;

fn compute_tripcode(trip: String) -> String {
    (sha256::digest(trip)[..TRIPCODE_LEN]).to_string()
}

fn actions_err(msg: &str) -> PlainchantErr {
    PlainchantErr {
        origin: ErrOrigin::Actions,
        msg:    String::from(msg),
    }
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

    pub fn is_banned(&self, ip: &str, cur_time: u64) -> Result<bool, PlainchantErr> {
        let rg = unwrap_or_return!(
            self.ban_cache.read(),
            Err(actions_err("Failed to read from Ban Cache"))
        );

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

        let mut wg = unwrap_or_return!(
            self.ban_cache.write(),
            Err(actions_err("Failed to write to Ban Cache"))
        );

        let ban = site::Ban {
            id: 0,
            ip: String::from(ip),
            time_expires,
        };

        wg.insert(String::from(ip), ban.clone());

        database.create_ban(ban)
    }

    pub fn unban_ip<DB: db::Database>(
        &self,
        database: &DB,
        ip: &str,
    ) -> Result<(), util::PlainchantErr> {
        database.delete_bans(ip)?;

        let mut wg = unwrap_or_return!(
            self.ban_cache.write(),
            Err(actions_err("Failed to write to Ban Cache"))
        );

        wg.remove(ip);

        Ok(())
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

        // This transaction also deletes replies
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

    pub fn delete_reply<DB: db::Database, FR: fr::FileRack>(
        &self,
        database: &DB,
        file_rack: &FR,
        board_id: u64,
        post_num: u64,
    ) -> Result<(), util::PlainchantErr> {
        let reply = database.get_reply(board_id, post_num)?;

        database.delete_reply(board_id, post_num)?;

        if let Some(id) = reply.file_id() {
            file_rack.delete_file(id)?;
        }

        Ok(())
    }

    pub fn delete_post<DB: db::Database, FR: fr::FileRack>(
        &self,
        database: &DB,
        file_rack: &FR,
        board_id: u64,
        post_num: u64,
    ) -> Result<(), util::PlainchantErr> {
        match database.get_thread(board_id, post_num) {
            Ok(_) => self.delete_thread(database, file_rack, board_id, post_num),
            Err(_) => self.delete_reply(database, file_rack, board_id, post_num),
        }
    }

    pub fn delete_all_posts_by_ip<DB: db::Database, FR: fr::FileRack>(
        &self,
        database: &DB,
        file_rack: &FR,
        ip: String,
    ) -> Result<usize, util::PlainchantErr> {
        let posts = database.get_all_posts_by_ip(ip)?;
        for post in &posts {
            // Allow this to error (double deletions)
            let _ = self.delete_post(database, file_rack, post.board_id(), post.post_num());
        }
        Ok(posts.len())
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
