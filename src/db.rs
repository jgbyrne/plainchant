use crate::site;

pub struct Thread {
    pub original: site::Original,
    pub replies : Vec<site::Reply>,
}

#[derive(Debug)]
pub struct DatabaseErr {
    msg: String,
}

pub fn static_err(msg: &'static str) -> DatabaseErr {
    DatabaseErr {
        msg: String::from(msg),
    }
}

trait Database {
    fn get_boards(&self) -> Vec<site::Board>;
    fn get_board(&self, board_id: u64) -> Result<site::Board, DatabaseErr>;
    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, DatabaseErr>;
    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<Thread, DatabaseErr>;
    fn get_original(&self, board_id: u64, post_num: u64) -> Result<site::Original, DatabaseErr>;
    fn get_reply(&self, board_id: u64, post_num: u64) -> Result<site::Reply, DatabaseErr>;
    fn get_post(&self, board_id: u64, post_num: u64) -> Result<Box<site::Post>, DatabaseErr>;
}
