use crate::db;
use crate::site;
use crate::util;
use std::path::{Path, PathBuf};
use std::fs::{read_to_string, read_dir};
use std::ffi::{OsString};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct FSDatabase {
    root: PathBuf,
    boards: Vec<(u64, String, String)>,
}

impl<'init> FSDatabase {
    pub fn from_root(root: &'init str) -> Result<FSDatabase, db::DatabaseErr> {
        let root_path = Path::new(&root).to_path_buf();
        let mut boards_path = root_path.join("boards");
        let mut boards_str = match read_to_string(boards_path) {
            Ok(boards_str) => boards_str,
            Err(read_err) => { return Err(db::static_err("Could not read"));  },
        };

        let mut boards = vec![];
        for line in boards_str.lines() {
            let parts = line.split(",").collect::<Vec<&str>>();
            if parts.len() == 3 {
                let id = match parts[0].parse::<u64>() {
                    Ok(id) => id,
                    Err(parse_err) => { return Err(db::static_err("Could not parse")); },
                };
                boards.push((id, parts[1].to_string(), parts[2].to_string()));
            }
            else {
                return Err(db::static_err("Too many parts"));
            }
        }

        Ok(FSDatabase { root: root_path, boards })
    }

    pub fn get_thread_reply(&self, board_id: u64, orig_num: u64, post_num: u64) -> Result<site::Reply, db::DatabaseErr> {
        let reply_path = self.root.join(board_id.to_string()).join(orig_num.to_string()).join(post_num.to_string());
        let mut post_str = match read_to_string(reply_path) {
            Ok(post_str) => post_str,
            Err(read_err) => { return Err(db::static_err("Could not retrieve reply post"));  },
        };
        let lines = post_str.lines().collect::<Vec<&str>>();
        if lines.len() < 4 {
            Err(db::static_err("Could not load reply post"))
        }
        else {
            let timestamp = match lines[0].parse::<u64>() {
                Ok(ts) => ts,
                Err(parse_err) => { return Err(db::static_err("Could not parse timestamp")); },
            };

            let poster = match lines[2] {
                 "" => None,
                 name @ _  => Some(name.to_string()),
            };

            Ok(site::Reply::new(
                board_id,
                post_num,
                timestamp,
                lines[1].to_string(),
                lines[3..].join("\n"),
                poster,
                None,
                None,
                orig_num,
            ))
        }
    }
}

impl db::Database for FSDatabase {
    fn get_boards(&self) -> Vec<site::Board> {
        self.boards.iter()
            .map(|b| site::Board { id: b.0, url: b.1.clone(), title: b.2.clone() } ).collect()
    }

    fn get_board(&self, board_id: u64) -> Result<site::Board, db::DatabaseErr> {
        for b in &self.boards {
            if b.0 == board_id {
                return Ok(site::Board { id: b.0, url: b.1.clone(), title: b.2.clone() });
            }
        }
        Err(db::static_err("No such board!"))
    }

    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, db::DatabaseErr> {
        let time = util::timestamp();
        let mut originals = vec![];
        // Hadouken!
        for entry in WalkDir::new(self.root.join(board_id.to_string())) {
            match entry {
                Ok(entry) => {
                    let e_path = entry.path();
                    if entry.depth() == 1 && e_path.is_dir() {
                        match e_path.file_name() {
                            Some(name) => match name.to_str() {
                                Some(name_str) => {
                                    match name_str.parse::<u64>() {
                                        Ok(orig_num) => {
                                            originals.push(
                                                self.get_original(board_id, orig_num)?);
                                        },
                                        Err(_) => {
                                            return Err(db::static_err(
                                                    "Could not parse directory name"));
                                        },
                                    }
                                },
                                None => continue,
                            },
                            None => continue,
                        }
                    }
                },
                Err(entry) => {
                    continue;
                }
            }
        }
        Ok(site::Catalog { board_id, time, originals })
    }
    
    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<db::Thread, db::DatabaseErr> {
        let thread_dir = self.root.join(board_id.to_string()).join(post_num.to_string());
        let dir_iter = match read_dir(thread_dir) {
            Ok(entries) => entries,
            Err(_) => return Err(db::static_err("Could not read thread directory")),
        };

        let original = self.get_original(board_id, post_num)?;
        let orig_filename = post_num.to_string();
        let mut replies = vec![];
        for entry in dir_iter {
            match entry {
                Ok(entry) => {
                    if !entry.path().is_file() {
                        continue;
                    }
                    let file = match entry.file_name().into_string() {
                        Ok(file) => file,
                        Err(_) => return Err(db::static_err("Could not understand filename")),
                    };
                    if file != orig_filename {
                        let reply_num = match file.parse::<u64>() {
                            Ok(num) => num,
                            Err(parse_err) => { return Err(
                                    db::static_err("Could not parse filename")); },
                        };
                        replies.push(self.get_thread_reply(board_id, post_num, reply_num)?);
                    }
                },
                Err(_) => return Err(db::static_err("Could not read thread dir entry")),
            }
        }
        Ok(db::Thread { original, replies } )
    }

    fn get_original(&self, board_id: u64, post_num: u64) -> Result<site::Original, db::DatabaseErr> {
        let orig_path = self.root.join(board_id.to_string())
                                 .join(post_num.to_string())
                                 .join(post_num.to_string());
        let mut post_str = match read_to_string(orig_path) {
            Ok(post_str) => post_str,
            Err(read_err) => { return Err(db::static_err("Could not retrieve original post"));  },
        };
        let lines = post_str.lines().collect::<Vec<&str>>();
        if lines.len() < 5 {
            Err(db::static_err("Could not load original post"))
        }
        else {
            let timestamp = match lines[0].parse::<u64>() {
                Ok(ts) => ts,
                Err(parse_err) => { return Err(db::static_err("Could not parse timestamp")); },
            };

            let poster = match lines[2] {
                 "" => None,
                 name @ _  => Some(name.to_string()),
            };

            let title = match lines[3] {
                 "" => None,
                 t @ _  => Some(t.to_string()),
            };

            Ok(site::Original::new(
                board_id,
                post_num,
                timestamp,
                lines[1].to_string(),
                lines[4..].join("\n"),
                poster,
                None,
                None,
                title,
                0,
                0,
            ))
        }
    }

    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, db::DatabaseErr> { 
        let post_filename = OsString::from(post_num.to_string());
        for entry in WalkDir::new(self.root.join(board_id.to_string())) {
            match entry {
                Ok(entry) => {
                    if entry.depth() == 2 {
                        let e_path = entry.path();
                        if e_path.is_file() && entry.file_name() == post_filename {
                            let thread_filename = e_path.parent().unwrap().file_name().unwrap();
                            let thread_str = thread_filename.to_string_lossy();
                            match thread_str.parse::<u64>() {
                                Ok(thread_num) =>
                                    return Ok(self.get_thread_reply(board_id, thread_num, post_num)?),
                                Err(_) =>
                                    return Err(db::static_err("Could not parse thread directory to number")),
                            }
                        }
                    }
                },
                Err(_) => {},
            }
        }
        Err(db::static_err("Could not find reply"))
    }
    
    fn get_post(&self, board_id: u64, post_num: u64) -> Result<Box<dyn site::Post>, db::DatabaseErr> {
        match self.get_original(board_id, post_num) {
            Ok(orig) => return Ok(Box::new(orig) as Box<dyn site::Post>),
            Err(_) => match self.get_reply(board_id, post_num) {
                Ok(reply) => return Ok(Box::new(reply) as Box<dyn site::Post>),
                Err(e) => Err(e),
            }
        }
    }

}
