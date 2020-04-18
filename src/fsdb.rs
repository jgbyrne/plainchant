use crate::db;
use crate::site;
use std::path::{Path, PathBuf};
use std::fs::{File, read_to_string};

#[derive(Debug)]
pub struct FSDatabase {
    root: PathBuf,
    boards: Vec<(u64, String, String)>,
}

impl<'init> FSDatabase {
    pub fn from_root(root: &'init str) -> Result<FSDatabase, db::DatabaseErr> {
        let root_path = Path::new(&root).to_path_buf();
        let mut board_file = root_path.join("boards");
        println!("{:?}", &board_file);
        let mut board_str = match read_to_string(board_file) {
            Ok(board_str) => board_str,
            Err(read_err) => { return Err(db::static_err("Could not read"));  },
        };

        let mut boards = vec![];
        for line in board_str.lines() {
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
}
