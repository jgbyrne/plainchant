use crate::site;
use crate::util;

#[derive(Debug)]
pub struct Thread {
    pub original: site::Original,
    pub replies:  Vec<site::Reply>,
}

#[allow(unused)]
pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr {
        origin: util::ErrOrigin::Database,
        msg:    String::from(msg),
    }
}

pub trait Database {
    fn get_site(&self) -> Result<site::Site, util::PlainchantErr>;
    fn set_site(&self, site: site::Site) -> Result<(), util::PlainchantErr>;

    fn get_boards(&self) -> Result<Vec<site::Board>, util::PlainchantErr>;
    fn get_board(&self, board_id: u64) -> Result<site::Board, util::PlainchantErr>;

    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, util::PlainchantErr>;

    fn get_original(
        &self,
        board_id: u64,
        post_num: u64,
    ) -> Result<site::Original, util::PlainchantErr>;

    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, util::PlainchantErr>;

    fn get_post(
        &self,
        board_id: u64,
        post_num: u64,
    ) -> Result<Box<dyn site::Post>, util::PlainchantErr>;

    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<Thread, util::PlainchantErr>;

    fn get_bans(&self) -> Result<Vec<site::Ban>, util::PlainchantErr>;

    // These two methods are called with dummy post IDs, which are auto-filled and returned
    fn create_original(&self, orig: site::Original) -> Result<u64, util::PlainchantErr>;
    fn create_reply(&self, reply: site::Reply) -> Result<u64, util::PlainchantErr>;

    fn delete_original(&self, board_id: u64, post_num: u64) -> Result<(), util::PlainchantErr>;
    fn delete_reply(&self, board_id: u64, post_num: u64) -> Result<(), util::PlainchantErr>;

    fn create_board(&self, board: site::Board) -> Result<(), util::PlainchantErr>;
    fn delete_board(&self, board_id: u64) -> Result<(), util::PlainchantErr>;

    fn create_ban(&self, ban: site::Ban) -> Result<(), util::PlainchantErr>;
    fn delete_bans(&self, ip: &str) -> Result<(), util::PlainchantErr>;
}
