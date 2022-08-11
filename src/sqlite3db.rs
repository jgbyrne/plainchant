use crate::db;
use crate::site;
use crate::util;
use crate::util::PlainchantErr;

use r2d2;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite;

use std::path::{Path, PathBuf};

impl From<rusqlite::Error> for PlainchantErr {
    fn from(err: rusqlite::Error) -> Self {
        PlainchantErr { msg:    format!("{}", err),
                        origin: util::ErrOrigin::Database, }
    }
}

impl From<r2d2::Error> for PlainchantErr {
    fn from(err: r2d2::Error) -> Self {
        PlainchantErr { msg:    format!("{}", err),
                        origin: util::ErrOrigin::Database, }
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

        conn.execute(
                     r#"
            CREATE TABLE IF NOT EXISTS Boards (
                BoardId     INTEGER  PRIMARY KEY,
                Url         TEXT     NOT NULL,
                Title       TEXT     NOT NULL,
                PostCap     INTEGER  NOT NULL,
                BumpLimit   INTEGER  NOT NULL,
                NextPostNum INTEGER  NOT NULL
            );
        "#,
                     (),
        )?;

        conn.execute(
                     r#"
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
                FileName    TEXT             ,
                OrigNum     INTEGER          ,
                PRIMARY KEY(BoardId, PostNum)
            );
        "#,
                     (),
        )?;

        conn.execute(
                     r#"
            CREATE TABLE IF NOT EXISTS Originals (
                BoardId     INTEGER  NUT NULL,
                PostNum     INTEGER  NOT NULL,
                Title       TEXT             ,
                BumpTime    INTEGER  NOT NULL,
                Replies     INTEGER  NOT NULL,
                ImgReplies  INTEGER  NOT NULL,
                Pinned      INTEGER  NOT NULL,
                Archived    INTEGER  NOT NULL,
                PRIMARY KEY(BoardId, PostNum)
            );
        "#,
                     (),
        )?;

        Ok(Sqlite3Database { path, pool })
    }
}

fn row_to_board<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Board> {
    Ok(site::Board { id: row.get(0)?,
                     url: row.get(1)?,
                     title: row.get(2)?,
                     post_cap: row.get(3)?,
                     bump_limit: row.get(4)?,
                     next_post_num: row.get(5)?, })
}

fn row_to_reply<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Reply> {
    let feather = match row.get::<usize, Option<u16>>(6)? {
        Some(1) => {
            match row.get(7)? {
                None => {
                    // this shouldn't really happen...
                    site::Feather::Trip(String::new())
                },
                Some(txt) => site::Feather::Trip(txt),
            }
        },
        Some(2) => site::Feather::Moderator,
        Some(3) => site::Feather::Admin,
        _ => site::Feather::None,
        // this should have error handling...
    };

    Ok(site::Reply { board_id: row.get(0)?,
                     post_num: row.get(1)?,
                     time: row.get(2)?,
                     ip: row.get(3)?,
                     body: row.get(4)?,
                     poster: row.get(5)?,
                     feather,
                     file_id: row.get(8)?,
                     file_name: row.get(9)?,
                     orig_num: row.get::<usize, Option<u64>>(10)?.unwrap_or(0) })
}

fn row_to_original<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Original> {
    let feather = match row.get::<usize, Option<u16>>(6)? {
        Some(1) => {
            match row.get(7)? {
                None => {
                    // this shouldn't really happen...
                    site::Feather::Trip(String::new())
                },
                Some(txt) => site::Feather::Trip(txt),
            }
        },
        Some(2) => site::Feather::Moderator,
        Some(3) => site::Feather::Admin,
        _ => site::Feather::None,
        // this should have error handling...
    };

    Ok(site::Original { board_id: row.get(0)?,
                        post_num: row.get(1)?,
                        time: row.get(2)?,
                        ip: row.get(3)?,
                        body: row.get(4)?,
                        poster: row.get(5)?,
                        feather,
                        file_id: row.get(8)?,
                        file_name: row.get(9)?,
                        title: row.get(10)?,
                        bump_time: row.get(11)?,
                        replies: row.get(12)?,
                        img_replies: row.get(13)?,
                        pinned: row.get(14)?,
                        archived: row.get(15)? })
}

impl db::Database for Sqlite3Database {
    fn get_boards(&self) -> Result<Vec<site::Board>, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT BoardId, Url, Title, PostCap, BumpLimit, NextPostNum FROM Boards;
        "#,
        )?;

        let boards_iter = query.query_map((), row_to_board)?;

        let mut boards = vec![];

        for b in boards_iter {
            boards.push(b?);
        }

        Ok(boards)
    }

    fn get_board(&self, board_id: u64) -> Result<site::Board, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT BoardId, Url, Title, PostCap, BumpLimit, NextPostNum FROM Boards
                WHERE BoardId=?1;
        "#,
        )?;

        query.query_row((board_id,), row_to_board)
             .map_err(|e| e.into())
    }

    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT p.BoardId, p.PostNum, p.Time, p.Ip, p.Poster, p.Body,
                   p.FeatherType, p.FeatherText, p.FileId, p.FileName,
                   o.Title, o.BumpTime, o.Replies, o.ImgReplies,
                   o.Pinned, o.Archived

            FROM   Posts p INNER JOIN Originals o
                        ON (p.BoardId, p.PostNum) = (o.BoardId, o.PostNum)

            WHERE p.BoardId = ?1;
        "#,
        )?;

        let orig_iter = query.query_map((board_id,), row_to_original)?;

        let mut originals = vec![];

        for o in orig_iter {
            originals.push(o?);
        }

        Ok(site::Catalog { board_id,
                           time: util::timestamp(),
                           originals })
    }

    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<db::Thread, PlainchantErr> {
        unimplemented!()
    }

    fn get_original(&self, board_id: u64, post_num: u64) -> Result<site::Original, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT p.BoardId, p.PostNum, p.Time, p.Ip, p.Poster, p.Body,
                   p.FeatherType, p.FeatherText, p.FileId, p.FileName,
                   o.Title, o.BumpTime, o.Replies, o.ImgReplies,
                   o.Pinned, o.Archived

            FROM   Posts p INNER JOIN Originals o
                        ON (p.BoardId, p.PostNum) = (o.BoardId, o.PostNum)

            WHERE (p.BoardId, p.PostNum) = (?1, ?2);
        "#,
        )?;

        query.query_row((board_id, post_num), row_to_original)
             .map_err(|e| e.into())
    }

    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT BoardId, PostNum, Time, Ip, Poster, Body,
                   FeatherType, FeatherText, FileId, FileName, OrigNum FROM Posts 
                WHERE (BoardId, PostNum) = (?1, ?2);
        "#,
        )?;

        // we use a Reply structure to fetch all posts, it won't matter when we cast to Post
        let post = query.query_row((board_id, post_num), row_to_reply)?;

        if post.orig_num == 1 {
            Err(PlainchantErr { origin: util::ErrOrigin::Database,
                                msg:    format!("Post ({}, {}) is an Original", board_id, post_num), })
        } else {
            Ok(post)
        }
    }

    fn get_post(&self, board_id: u64, post_num: u64) -> Result<Box<dyn site::Post>, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
                                     r#"
            SELECT BoardId, PostNum, Time, Ip, Poster, Body,
                   FeatherType, FeatherText, FileId, FileName, OrigNum FROM Posts 
                WHERE (BoardId, PostNum)=(?1, ?2);
        "#,
        )?;

        // we use a Reply structure to fetch all posts, it won't matter when we cast to Post
        let post = query.query_row((board_id, post_num), row_to_reply)?;

        Ok(Box::new(post) as Box<dyn site::Post>)
    }

    fn create_original(&mut self, orig: site::Original) -> Result<u64, PlainchantErr> {
        unimplemented!()
    }
    fn create_reply(&mut self, reply: site::Reply) -> Result<u64, PlainchantErr> {
        unimplemented!()
    }

    fn update_original(&mut self, orig: site::Original) -> Result<(), PlainchantErr> {
        unimplemented!()
    }
    fn update_reply(&mut self, reply: site::Reply) -> Result<(), PlainchantErr> {
        unimplemented!()
    }

    fn delete_original(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> {
        unimplemented!()
    }
    fn delete_reply(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> {
        unimplemented!()
    }
    fn delete_post(&mut self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> {
        unimplemented!()
    }
}
