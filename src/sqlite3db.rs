use crate::db;
use crate::site;
use crate::util;
use crate::util::PlainchantErr;

use r2d2;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite;

use core::ops::Deref;
use std::path::PathBuf;

impl From<rusqlite::Error> for PlainchantErr {
    fn from(err: rusqlite::Error) -> Self {
        PlainchantErr {
            msg:    format!("{}", err),
            origin: util::ErrOrigin::Database,
        }
    }
}

impl From<r2d2::Error> for PlainchantErr {
    fn from(err: r2d2::Error) -> Self {
        PlainchantErr {
            msg:    format!("{}", err),
            origin: util::ErrOrigin::Database,
        }
    }
}

fn encode_feather<'f>(feather: &'f site::Feather) -> (Option<u8>, Option<&'f str>) {
    match feather {
        site::Feather::None => (None, None),
        site::Feather::Trip(ref s) => (Some(1), Some(s)),
        site::Feather::Moderator => (Some(2), None),
        site::Feather::Admin => (Some(3), None),
    }
}

fn decode_feather(feather_type: Option<u16>, feather_text: Option<String>) -> site::Feather {
    // This really should fail for (Some(1), None) and for (Some(x), *) : x not in {1, 2, 3}
    // However I do not know a clean way to generate my own errors in rusqlite handlers
    // I don't think it'll be a problem, anyway...
    match feather_type {
        Some(1) => match feather_text {
            Some(txt) => site::Feather::Trip(txt),
            None => site::Feather::Trip(String::new()),
        },
        Some(2) => site::Feather::Moderator,
        Some(3) => site::Feather::Admin,
        _ => site::Feather::None,
    }
}

pub struct Sqlite3Database {
    #[allow(unused)]
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
            CREATE TABLE IF NOT EXISTS Site (
                Identity    INTEGER  PRIMARY KEY,
                Name        TEXT     NOT NULL,
                Description TEXT     NOT NULL
            );
        "#,
            (),
        )?;

        conn.execute(
            r#"
            INSERT OR IGNORE INTO Site VALUES (
                1,
                'Plainchant',
                'A lightweight and libre imageboard.'
            );
        "#,
            (),
        )?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS Bans (
                BanId       INTEGER  PRIMARY KEY,
                Ip          TEXT     NOT NULL,
                TimeExpires INTEGER  NOT NULL
            );
        "#,
            (),
        )?;

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

fn row_to_ban<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Ban> {
    Ok(site::Ban {
        id:           row.get(0)?,
        ip:           row.get(1)?,
        time_expires: row.get(2)?,
    })
}

fn row_to_board<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Board> {
    Ok(site::Board {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        post_cap: row.get(3)?,
        bump_limit: row.get(4)?,
        next_post_num: row.get(5)?,
    })
}

fn row_to_reply<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Reply> {
    let feather = decode_feather(row.get::<usize, Option<u16>>(6)?, row.get(7)?);

    Ok(site::Reply {
        board_id: row.get(0)?,
        post_num: row.get(1)?,
        time: row.get(2)?,
        ip: row.get(3)?,
        poster: row.get(4)?,
        body: row.get(5)?,
        feather,
        file_id: row.get(8)?,
        file_name: row.get(9)?,
        orig_num: row.get::<usize, Option<u64>>(10)?.unwrap_or(0),
    })
}

fn row_to_original<'stmt>(row: &rusqlite::Row<'stmt>) -> rusqlite::Result<site::Original> {
    let feather = decode_feather(row.get::<usize, Option<u16>>(6)?, row.get(7)?);

    Ok(site::Original {
        board_id: row.get(0)?,
        post_num: row.get(1)?,
        time: row.get(2)?,
        ip: row.get(3)?,
        poster: row.get(4)?,
        body: row.get(5)?,
        feather,
        file_id: row.get(8)?,
        file_name: row.get(9)?,
        title: row.get(10)?,
        bump_time: row.get(11)?,
        replies: row.get(12)?,
        img_replies: row.get(13)?,
        pinned: row.get(14)?,
        archived: row.get(15)?,
    })
}

fn query_board<T: Deref<Target = rusqlite::Connection>>(
    conn: &T,
    board_id: u64,
) -> Result<site::Board, PlainchantErr> {
    let mut query = conn.prepare(
        r#"
            SELECT BoardId, Url, Title, PostCap, BumpLimit, NextPostNum FROM Boards
                WHERE BoardId=?1;
        "#,
    )?;

    query
        .query_row((board_id,), row_to_board)
        .map_err(|e| e.into())
}

fn query_original<T: Deref<Target = rusqlite::Connection>>(
    conn: &T,
    board_id: u64,
    post_num: u64,
) -> Result<site::Original, PlainchantErr> {
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

    query
        .query_row((board_id, post_num), row_to_original)
        .map_err(|e| e.into())
}

fn query_reply<T: Deref<Target = rusqlite::Connection>>(
    conn: &T,
    board_id: u64,
    post_num: u64,
) -> Result<site::Reply, PlainchantErr> {
    let mut query = conn.prepare(
        r#"
        SELECT BoardId, PostNum, Time, Ip, Poster, Body,
               FeatherType, FeatherText, FileId, FileName, OrigNum FROM Posts 
            WHERE (BoardId, PostNum) = (?1, ?2);
    "#,
    )?;

    let post = query.query_row((board_id, post_num), row_to_reply)?;

    if post.orig_num == 1 {
        Err(PlainchantErr {
            origin: util::ErrOrigin::Database,
            msg:    format!("Post ({}, {}) is an Original", board_id, post_num),
        })
    } else {
        Ok(post)
    }
}

fn increment_next_post_num<T: Deref<Target = rusqlite::Connection>>(
    conn: &T,
    board_id: u64,
) -> Result<(), PlainchantErr> {
    conn.execute(
        r#"
                UPDATE Boards
                SET NextPostNum = NextPostNum + 1
                WHERE BoardId = ?1;
                "#,
        (board_id,),
    )?;

    Ok(())
}

impl db::Database for Sqlite3Database {
    fn get_site(&self) -> Result<site::Site, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
            r#"
            SELECT Name, Description FROM Site
            WHERE Identity = 1;
            "#,
        )?;

        let site = query.query_row((), |row| {
            Ok(site::Site {
                name:        row.get(0)?,
                description: row.get(1)?,
            })
        })?;

        Ok(site)
    }

    fn set_site(&self, site: site::Site) -> Result<(), PlainchantErr> {
        let conn = self.pool.get()?;
        conn.execute(
            r#"
            REPLACE INTO Site VALUES (
                1,
                ?1,
                ?2
            );
            "#,
            (site.name, site.description),
        )?;

        Ok(())
    }

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
        query_board(&conn, board_id)
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

            WHERE p.BoardId = ?1
            ORDER BY o.BumpTime DESC;
        "#,
        )?;

        let orig_iter = query.query_map((board_id,), row_to_original)?;

        let mut originals = vec![];

        for o in orig_iter {
            originals.push(o?);
        }

        Ok(site::Catalog {
            board_id,
            time: util::timestamp(),
            originals,
        })
    }

    fn get_original(&self, board_id: u64, post_num: u64) -> Result<site::Original, PlainchantErr> {
        let conn = self.pool.get()?;
        query_original(&conn, board_id, post_num)
    }

    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<db::Thread, PlainchantErr> {
        let conn = self.pool.get()?;
        let original = query_original(&conn, board_id, post_num)?;

        let mut replies_query = conn.prepare(
            r#"
            SELECT BoardId, PostNum, Time, Ip, Poster, Body,
                   FeatherType, FeatherText, FileId, FileName, OrigNum FROM Posts 
                WHERE (BoardId, OrigNum) = (?1, ?2);
        "#,
        )?;

        let replies_iter = replies_query.query_map((board_id, post_num), row_to_reply)?;
        let mut replies = vec![];
        for r in replies_iter {
            replies.push(r?);
        }

        Ok(db::Thread { original, replies })
    }

    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, PlainchantErr> {
        let conn = self.pool.get()?;
        query_reply(&conn, board_id, post_num)
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

    fn get_bans(&self) -> Result<Vec<site::Ban>, PlainchantErr> {
        let conn = self.pool.get()?;
        let mut query = conn.prepare(
            r#"
            SELECT BanId, Ip, TimeExpires FROM Bans
        "#,
        )?;

        let bans_iter = query.query_map((), row_to_ban)?;

        let mut bans = vec![];

        for b in bans_iter {
            bans.push(b?);
        }

        Ok(bans)
    }

    fn create_original(&self, mut orig: site::Original) -> Result<u64, PlainchantErr> {
        let mut conn = self.pool.get()?;

        let mut board = query_board(&conn, orig.board_id)?;
        orig.post_num = board.next_post_num;
        board.next_post_num += 1;

        let tx = conn.transaction()?;
        increment_next_post_num(&tx, orig.board_id)?;

        let (feather_type, feather_text) = encode_feather(&orig.feather);

        tx.execute(
            r#"
            INSERT INTO Posts
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL);
            "#,
            (
                orig.board_id,
                orig.post_num,
                orig.time,
                &orig.ip,
                &orig.poster,
                &orig.body,
                feather_type,
                feather_text,
                &orig.file_id,
                &orig.file_name,
            ),
        )?;

        tx.execute(
            r#"
            INSERT INTO Originals
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);
            "#,
            (
                orig.board_id,
                orig.post_num,
                &orig.title,
                orig.bump_time,
                orig.replies,
                orig.img_replies,
                orig.pinned,
                orig.archived,
            ),
        )?;

        tx.commit()?;
        Ok(orig.post_num)
    }

    fn create_reply(&self, mut reply: site::Reply) -> Result<u64, PlainchantErr> {
        let mut conn = self.pool.get()?;

        let mut board = query_board(&conn, reply.board_id)?;
        reply.post_num = board.next_post_num;
        board.next_post_num += 1;

        let orig = query_original(&conn, reply.board_id, reply.orig_num)?;

        let new_reply_count = orig.replies + 1;
        let new_img_reply_count = if reply.file_id.is_some() {
            orig.img_replies + 1
        } else {
            orig.img_replies
        };

        let new_bump_time = if new_reply_count <= board.bump_limit {
            util::timestamp()
        } else {
            orig.bump_time
        };

        let tx = conn.transaction()?;
        increment_next_post_num(&tx, reply.board_id)?;

        let (feather_type, feather_text) = encode_feather(&reply.feather);

        tx.execute(
            r#"
            INSERT INTO Posts
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);
            "#,
            (
                reply.board_id,
                reply.post_num,
                reply.time,
                &reply.ip,
                &reply.poster,
                &reply.body,
                feather_type,
                feather_text,
                &reply.file_id,
                &reply.file_name,
                orig.post_num,
            ),
        )?;

        tx.execute(
            r#"
            UPDATE Originals
            SET BumpTime = ?3, Replies = ?4, ImgReplies = ?5
            WHERE (BoardId, PostNum) = (?1, ?2);
            "#,
            (
                reply.board_id,
                orig.post_num,
                new_bump_time,
                new_reply_count,
                new_img_reply_count,
            ),
        )?;

        tx.commit()?;
        Ok(reply.post_num)
    }

    fn delete_original(&self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;

        tx.execute(
            r#"
            DELETE FROM Posts WHERE (BoardId, PostNum)=(?1, ?2);
            "#,
            (board_id, post_num),
        )?;

        tx.execute(
            r#"
            DELETE FROM Originals WHERE (BoardId, PostNum)=(?1, ?2);
            "#,
            (board_id, post_num),
        )?;

        tx.execute(
            r#"
            DELETE FROM Posts WHERE (BoardId, OrigNum)=(?1, ?2);
            "#,
            (board_id, post_num),
        )?;

        tx.commit()?;

        Ok(())
    }

    fn delete_reply(&self, board_id: u64, post_num: u64) -> Result<(), PlainchantErr> {
        let mut conn = self.pool.get()?;

        let reply = query_reply(&conn, board_id, post_num)?;
        let orig = query_original(&conn, board_id, reply.orig_num)?;

        let new_reply_count = orig.replies - 1;
        let new_img_reply_count = if reply.file_id.is_some() {
            orig.img_replies - 1
        } else {
            orig.img_replies
        };

        let tx = conn.transaction()?;

        tx.execute(
            r#"
            DELETE FROM Posts WHERE (BoardId, PostNum)=(?1, ?2);
            "#,
            (board_id, post_num),
        )?;

        tx.execute(
            r#"
            UPDATE Originals
            SET Replies = ?3, ImgReplies = ?4
            WHERE (BoardId, PostNum) = (?1, ?2);
            "#,
            (
                reply.board_id,
                orig.post_num,
                new_reply_count,
                new_img_reply_count,
            ),
        )?;

        tx.commit()?;

        Ok(())
    }

    fn create_board(&self, board: site::Board) -> Result<(), PlainchantErr> {
        let conn = self.pool.get()?;

        conn.execute(
            r#"
            INSERT INTO Boards
            VALUES (?1, ?2, ?3, ?4, ?5, ?6);
            "#,
            (
                board.id,
                board.url,
                board.title,
                board.post_cap,
                board.bump_limit,
                board.next_post_num,
            ),
        )?;

        Ok(())
    }

    fn delete_board(&self, board_id: u64) -> Result<(), PlainchantErr> {
        let mut conn = self.pool.get()?;

        let tx = conn.transaction()?;

        tx.execute("DELETE FROM Posts WHERE BoardId = ?1;", (board_id,))?;
        tx.execute("DELETE FROM Originals WHERE BoardId = ?1;", (board_id,))?;
        tx.execute("DELETE FROM Boards WHERE BoardId = ?1;", (board_id,))?;

        tx.commit()?;

        Ok(())
    }

    fn create_ban(&self, ban: site::Ban) -> Result<(), PlainchantErr> {
        let conn = self.pool.get()?;

        conn.execute(
            r#"
            INSERT INTO Bans
            (Ip, TimeExpires)
            VALUES (?1, ?2);
            "#,
            (ban.ip, ban.time_expires),
        )?;

        Ok(())
    }

    fn delete_bans(&self, ip: &str) -> Result<(), PlainchantErr> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM Bans WHERE Ip = ?1;", (ip,))?;
        Ok(())
    }
}
