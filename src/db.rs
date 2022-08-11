use crate::site;
use crate::util;

#[derive(Debug)]
pub struct Thread {
    pub original: site::Original,
    pub replies:  Vec<site::Reply>,
}

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr {
        origin: util::ErrOrigin::Database,
        msg:    String::from(msg),
    }
}

pub trait Database {
    fn get_boards(&self) -> Result<Vec<site::Board>, util::PlainchantErr>;
    fn get_board(&self, board_id: u64) -> Result<site::Board, util::PlainchantErr>;
    fn get_catalog(&self, board_id: u64) -> Result<site::Catalog, util::PlainchantErr>;
    fn get_thread(&self, board_id: u64, post_num: u64) -> Result<Thread, util::PlainchantErr>;

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

    // These methods are called with dummy post IDs, which are auto-filled and returned
    fn create_original(&mut self, orig: site::Original) -> Result<u64, util::PlainchantErr>;
    fn create_reply(&mut self, reply: site::Reply) -> Result<u64, util::PlainchantErr>;

    //   fn update_original(&mut self, orig: site::Original) -> Result<(), util::PlainchantErr>;
    //fn update_reply(&mut self, reply: site::Reply) -> Result<(), util::PlainchantErr>;

    fn delete_original(&mut self, board_id: u64, post_num: u64) -> Result<(), util::PlainchantErr>;
    fn delete_reply(&mut self, board_id: u64, post_num: u64) -> Result<(), util::PlainchantErr>;
    //    fn delete_post(&mut self, board_id: u64, post_num: u64) -> Result<(), util::PlainchantErr>;

    fn create_board(&mut self, board: site::Board) -> Result<(), util::PlainchantErr>;
    fn delete_board(&mut self, board_id: u64) -> Result<(), util::PlainchantErr>;
}
