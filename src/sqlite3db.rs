use crate::db;
use crate::site;
use crate::util;
use crate::util::PlainchantErr;

use rusqlite;
use r2d2;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use std::path::{Path, PathBuf};

impl From<rusqlite::Error> for PlainchantErr {
    fn from(err: rusqlite::Error) -> Self {
        PlainchantErr {
            msg: format!("{}", err),
            origin: util::ErrOrigin::Database,
        }
    }
}

impl From<r2d2::Error> for PlainchantErr {
    fn from(err: r2d2::Error) -> Self {
        PlainchantErr {
            msg: format!("{}", err),
            origin: util::ErrOrigin::Database,
        }
    }
}

pub struct Sqlite3Database {
    path: PathBuf,
    pool: Pool<SqliteConnectionManager>,
}

impl Sqlite3Database {
    pub fn from_path(path: PathBuf) -> Result<Self, PlainchantErr> {
        let manager = SqliteConnectionManager::file(&path);
        let pool = r2d2::Pool::new(manager)?;

        Ok(Sqlite3Database { path, pool })
    }
}

impl db::Database for Sqlite3Database {
    fn get_boards(&self) -> Result<Vec<site::Board>, PlainchantErr> { unimplemented!() }
    fn get_board(&self, board_id: u64) -> Result<site::Board, PlainchantErr> { unimplemented!() }
    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, PlainchantErr> { unimplemented!() }
    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<db::Thread, PlainchantErr> { unimplemented!() }

    fn get_original(&self,
                    board_id: u64,
                    post_num: u64)
                    -> Result<site::Original, PlainchantErr> { unimplemented!() }
    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, PlainchantErr> { unimplemented!() }
    fn get_post(&self,
                board_id: u64,
                post_num: u64)
                -> Result<Box<dyn site::Post>, PlainchantErr> { unimplemented!() }

    fn create_original(&mut self, orig: site::Original) -> Result<u64, PlainchantErr> { unimplemented!() }
    fn create_reply(&mut self, reply: site::Reply) -> Result<u64, PlainchantErr> { unimplemented!() }

    fn update_original(&mut self, orig: site::Original) -> Result<(), PlainchantErr> { unimplemented!() }
    fn update_reply(&mut self, reply: site::Reply) -> Result<(), PlainchantErr> { unimplemented!() }

    fn delete_original(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> { unimplemented!() }
    fn delete_reply(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> { unimplemented!() }
    fn delete_post(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> { unimplemented!() }
}

