use crate::site;
use crate::util;

#[derive(Debug)]
pub struct Thread {
    pub original: site::Original,
    pub replies : Vec<site::Reply>,
}

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr {
        origin: util::ErrOrigin::Database,
        msg: String::from(msg),
    }
}

pub trait Database {
    fn get_boards(&self) -> Vec<site::Board>;
    fn get_board(&self, board_id: u64) -> Result<site::Board, util::PlainchantErr>;
    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, util::PlainchantErr>;
    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<Thread, util::PlainchantErr>;
    fn get_original(&self, board_id: u64, post_num: u64) -> Result<site::Original, util::PlainchantErr>;
    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, util::PlainchantErr>;
    fn get_post(&self, board_id: u64, post_num: u64) -> Result<Box<dyn site::Post>, util::PlainchantErr>;
}
