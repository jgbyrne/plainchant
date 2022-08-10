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

        let conn = pool.get()?;

        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS Boards (
                BoardId     INTEGER  PRIMARY KEY,
                Url         TEXT     NOT NULL,
                Title       TEXT     NOT NULL,
                PostCap     INTEGER  NOT NULL,
                BumpLimit   INTEGER  NOT NULL,
                NextPostNum INTEGER  NOT NULL
            );
        "#, ())?;

        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS Posts (
                BoardId     INTEGER  NUT NULL,
                PostNum     INTEGER  NOT NULL,
                Time        INTEGER  NOT NULL,
                Ip          TEXT     NOT NULL,
                Poster      TEXT             ,
                Body        TEXT     NOT NULL,
                FeatherType INTEGER          ,
                FeatherText TEXT             ,
                FileId      TEXT             ,
                ReplyTo     INTEGER          ,
                PRIMARY KEY(BoardId, PostNum)
            );
        "#, ())?;

        conn.execute(r#"
            CREATE TABLE IF NOT EXISTS Originals (
                BoardId     INTEGER  NUT NULL,
                PostNum     INTEGER  NOT NULL,
                Title       TEXT             ,
                Replies     INTEGER  NOT NULL,
                ImgReplies  INTEGER  NOT NULL,
                PRIMARY KEY(BoardId, PostNum)
            );
        "#, ())?;

        Ok(Sqlite3Database { path, pool })
    }
}

impl db::Database for Sqlite3Database {
    fn get_boards(&self) -> Result<Vec<site::Board>, PlainchantErr> { 
        let conn = self.pool.get()?;
        let mut query = conn.prepare(r#"
            SELECT BoardId, Url, Title, PostCap, BumpLimit, NextPostNum FROM Boards;
        "#)?;

        let boards_iter = query.query_map((), |row| {
            Ok(site::Board {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                post_cap: row.get(3)?,
                bump_limit: row.get(4)?,
                next_post_num: row.get(5)?,
            })
        })?;

        let mut boards = vec![];

        for b in boards_iter {
            boards.push(b?);
        }

        Ok(boards)
    }

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

