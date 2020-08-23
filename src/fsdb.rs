use crate::db;
use crate::site;
use crate::site::Post;
use crate::util;
use std::ffi::OsString;
use std::fs::{create_dir, read_dir, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct FSDatabase {
    root:        PathBuf,
    boards_path: PathBuf,
    boards:      Vec<(u64, String, String, u64)>,
}

impl<'init> FSDatabase {
    pub fn from_root(root: &'init str) -> Result<FSDatabase, util::PlainchantErr> {
        let root_path = Path::new(&root).to_path_buf();
        let boards_path = root_path.join("boards");
        let boards_str = match read_to_string(&boards_path) {
            Ok(boards_str) => boards_str,
            Err(_read_err) => {
                return Err(db::static_err("Could not read"));
            },
        };

        let mut boards = vec![];
        for line in boards_str.lines() {
            let parts = line.split(",").collect::<Vec<&str>>();
            if parts.len() == 4 {
                let id = match parts[0].parse::<u64>() {
                    Ok(id) => id,
                    Err(_parse_err) => {
                        return Err(db::static_err("Could not parse board id"));
                    },
                };
                let next_post_num = match parts[3].parse::<u64>() {
                    Ok(num) => num,
                    Err(_parse_err) => {
                        return Err(db::static_err("Could not parse next post_num"));
                    },
                };
                boards.push((id, parts[1].to_string(), parts[2].to_string(), next_post_num));
            } else {
                return Err(db::static_err("Too many parts"));
            }
        }

        Ok(FSDatabase { root: root_path,
                        boards_path,
                        boards })
    }

    pub fn write_boards_file(&self) -> Result<(), util::PlainchantErr> {
        let mut boards_str = String::new();
        for board in self.boards.iter() {
            boards_str.push_str(&format!("{},{},{},{}\n", board.0, board.1, board.2, board.3));
        }
        if let Ok(mut file) = File::create(&self.boards_path) {
            match file.write_all(boards_str.as_bytes()) {
                Ok(_) => Ok(()),
                Err(_) => Err(db::static_err("Could not write to boards file")),
            }
        } else {
            Err(db::static_err("Could not open boards file for writing"))
        }
    }

    pub fn use_next_post_num(&mut self, board_id: u64) -> Result<u64, util::PlainchantErr> {
        for mut board in self.boards.iter_mut() {
            if board.0 == board_id {
                let next = board.3;
                board.3 += 1;
                return Ok(next);
            }
        }
        Err(db::static_err("No such board"))
    }

    pub fn get_thread_reply(&self,
                            board_id: u64,
                            orig_num: u64,
                            post_num: u64)
                            -> Result<site::Reply, util::PlainchantErr> {
        let reply_path = self.root
                             .join(board_id.to_string())
                             .join(orig_num.to_string())
                             .join(post_num.to_string());
        let post_str = match read_to_string(reply_path) {
            Ok(post_str) => post_str,
            Err(_read_err) => {
                return Err(db::static_err("Could not retrieve reply post"));
            },
        };
        let lines = post_str.lines().collect::<Vec<&str>>();
        if lines.len() < 4 {
            Err(db::static_err("Could not load reply post"))
        } else {
            let timestamp = match lines[0].parse::<u64>() {
                Ok(ts) => ts,
                Err(_parse_err) => {
                    return Err(db::static_err("Could not parse timestamp"));
                },
            };

            let poster = match lines[2] {
                "" => None,
                name @ _ => Some(name.to_string()),
            };

            let file_id = match lines[3] {
                "" => None,
                file_id @ _ => Some(file_id.to_string()),
            };

            Ok(site::Reply::new(board_id,
                                post_num,
                                timestamp,
                                lines[1].to_string(),
                                lines[5..].join("\n"),
                                poster,
                                file_id,
                                Some(lines[4].to_string()), // file name
                                orig_num))
        }
    }

    fn serialise_original(orig: &site::Original) -> String {
        let mut data = String::new();
        data.push_str(&format!("{}\n", orig.time()));
        data.push_str(&format!("{}\n", orig.bump_time()));
        data.push_str(&format!("{}\n", orig.ip()));
        data.push_str(&format!("{}\n", orig.poster().unwrap_or("")));
        data.push_str(&format!("{}\n", orig.title().unwrap_or("")));
        data.push_str(&format!("{}\n", orig.file_id().unwrap_or("")));
        data.push_str(&format!("{}\n", orig.file_name().unwrap_or("")));
        data.push_str(&format!("{}\n", orig.replies()));
        data.push_str(&format!("{}\n", orig.img_replies()));
        data.push_str(&orig.body());

        data
    }
}

impl db::Database for FSDatabase {
    fn get_boards(&self) -> Vec<site::Board> {
        self.boards
            .iter()
            .map(|b| site::Board { id:    b.0,
                                   url:   b.1.clone(),
                                   title: b.2.clone(), })
            .collect()
    }

    fn get_board(&self, board_id: u64) -> Result<site::Board, util::PlainchantErr> {
        for b in &self.boards {
            if b.0 == board_id {
                return Ok(site::Board { id:    b.0,
                                        url:   b.1.clone(),
                                        title: b.2.clone(), });
            }
        }
        Err(db::static_err("No such board!"))
    }

    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, util::PlainchantErr> {
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
                                Some(name_str) => match name_str.parse::<u64>() {
                                    Ok(orig_num) => {
                                        originals.push(self.get_original(board_id, orig_num)?);
                                    },
                                    Err(_) => {
                                        return Err(db::static_err(
                                            "Could not parse directory name",
                                        ));
                                    },
                                },
                                None => continue,
                            },
                            None => continue,
                        }
                    }
                },
                Err(_entry) => {
                    continue;
                },
            }
        }
        originals.sort_unstable_by_key(|orig| u64::max_value() - orig.bump_time());
        Ok(site::Catalog { board_id,
                           time,
                           originals })
    }

    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<db::Thread, util::PlainchantErr> {
        let thread_dir = self.root
                             .join(board_id.to_string())
                             .join(post_num.to_string());
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
                            Err(_parse_err) => {
                                return Err(db::static_err("Could not parse filename"));
                            },
                        };
                        replies.push(self.get_thread_reply(board_id, post_num, reply_num)?);
                    }
                },
                Err(_) => return Err(db::static_err("Could not read thread dir entry")),
            }
        }
        Ok(db::Thread { original, replies })
    }

    fn get_original(&self,
                    board_id: u64,
                    post_num: u64)
                    -> Result<site::Original, util::PlainchantErr> {
        let orig_path = self.root
                            .join(board_id.to_string())
                            .join(post_num.to_string())
                            .join(post_num.to_string());
        let post_str = match read_to_string(orig_path) {
            Ok(post_str) => post_str,
            Err(_read_err) => {
                return Err(db::static_err("Could not retrieve original post"));
            },
        };
        let lines = post_str.lines().collect::<Vec<&str>>();
        if lines.len() < 5 {
            Err(db::static_err("Could not load original post"))
        } else {
            let timestamp = match lines[0].parse::<u64>() {
                Ok(ts) => ts,
                Err(_parse_err) => {
                    return Err(db::static_err("Could not parse timestamp"));
                },
            };

            let bump_time = match lines[1].parse::<u64>() {
                Ok(ts) => ts,
                Err(_parse_err) => {
                    return Err(db::static_err("Could not parse bump time"));
                },
            };

            let poster = match lines[3] {
                "" => None,
                name @ _ => Some(name.to_string()),
            };

            let title = match lines[4] {
                "" => None,
                t @ _ => Some(t.to_string()),
            };

            let replies = match lines[7].parse::<u16>() {
                Ok(r) => r,
                Err(_parse_err) => {
                    return Err(db::static_err("Could not parse replies count"));
                },
            };

            let img_replies = match lines[8].parse::<u16>() {
                Ok(ir) => ir,
                Err(_parse_err) => {
                    return Err(db::static_err("Could not parse image replies count"));
                },
            };

            Ok(site::Original::new(board_id,
                                   post_num,
                                   timestamp,
                                   lines[2].to_string(),  // ip
                                   lines[9..].join("\n"), // body
                                   poster,
                                   Some(lines[5].to_string()), // file ID
                                   Some(lines[6].to_string()), // file name
                                   title,
                                   bump_time,
                                   replies,
                                   img_replies))
        }
    }

    fn update_original(&mut self, orig: site::Original) -> Result<(), util::PlainchantErr> {
        let board_path = self.root.join(orig.board_id().to_string());
        if !board_path.exists() {
            return Err(db::static_err("No such board"));
        }

        let thread_path = board_path.join(orig.post_num().to_string());
        if !thread_path.exists() {
            return Err(db::static_err("Thread does not exist"));
        }

        let data = FSDatabase::serialise_original(&orig);

        // Write post file to disk
        if let Ok(mut file) = File::create(thread_path.join(orig.post_num().to_string())) {
            if file.write_all(data.as_bytes()).is_err() {
                return Err(db::static_err("Error writing post file"));
            }
        } else {
            return Err(db::static_err("Unable to write post file"));
        }

        self.write_boards_file()?;
        Ok(())
    }

    fn create_original(&mut self, mut orig: site::Original) -> Result<u64, util::PlainchantErr> {
        // Create thread directory
        let board_path = self.root.join(orig.board_id().to_string());
        if !board_path.exists() {
            return Err(db::static_err("No such board"));
        }

        let post_num = self.use_next_post_num(orig.board_id())?;
        orig.set_post_num(post_num);

        let thread_path = board_path.join(post_num.to_string());
        if thread_path.exists() {
            return Err(db::static_err("Thread already exists"));
        }

        let dir_creation = create_dir(&thread_path);
        if !dir_creation.is_ok() {
            return Err(db::static_err("Could not create thread directory"));
        }

        let data = FSDatabase::serialise_original(&orig);

        // Write post file to disk
        if let Ok(mut file) = File::create(thread_path.join(orig.post_num().to_string())) {
            if file.write_all(data.as_bytes()).is_err() {
                return Err(db::static_err("Error writing post file"));
            }
        } else {
            return Err(db::static_err("Unable to create post file"));
        }

        self.write_boards_file()?;
        Ok(post_num)
    }

    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, util::PlainchantErr> {
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
                                Ok(thread_num) => {
                                    return Ok(
                                        self.get_thread_reply(board_id, thread_num, post_num)?
                                    )
                                }
                                Err(_) => {
                                    return Err(db::static_err(
                                        "Could not parse thread directory to number",
                                    ))
                                }
                            }
                        }
                    }
                },
                Err(_) => {},
            }
        }
        Err(db::static_err("Could not find reply"))
    }

    fn create_reply(&mut self, mut reply: site::Reply) -> Result<u64, util::PlainchantErr> {
        let post_num = self.use_next_post_num(reply.board_id())?;
        reply.set_post_num(post_num);

        let thread_path = self.root
                              .join(reply.board_id().to_string())
                              .join(reply.orig_num().to_string());
        if !thread_path.exists() {
            return Err(db::static_err("Thread does not exist"));
        }

        // Compose post file
        let mut data = String::new();
        data.push_str(&format!("{}\n", reply.time()));
        data.push_str(&format!("{}\n", reply.ip()));
        data.push_str(&format!("{}\n", reply.poster().unwrap_or("")));
        data.push_str(&format!("{}\n", reply.file_id().unwrap_or("")));
        data.push_str(&format!("{}\n", reply.file_name().unwrap_or("")));
        data.push_str(&reply.body());

        // Write post file to disk
        if let Ok(mut file) = File::create(thread_path.join(reply.post_num().to_string())) {
            if file.write_all(data.as_bytes()).is_err() {
                return Err(db::static_err("Error writing post file"));
            }
        } else {
            return Err(db::static_err("Unable to create post file"));
        }

        self.write_boards_file()?;
        Ok(post_num)
    }

    fn get_post(&self,
                board_id: u64,
                post_num: u64)
                -> Result<Box<dyn site::Post>, util::PlainchantErr> {
        match self.get_original(board_id, post_num) {
            Ok(orig) => return Ok(Box::new(orig) as Box<dyn site::Post>),
            Err(_) => match self.get_reply(board_id, post_num) {
                Ok(reply) => return Ok(Box::new(reply) as Box<dyn site::Post>),
                Err(e) => Err(e),
            },
        }
    }
}
