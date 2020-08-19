use crate::site;
use crate::util;

use bytes::Bytes;

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    util::PlainchantErr {
        origin: util::ErrOrigin::FileRack,
        msg: String::from(msg),
    }
}

pub trait FileRack {
    fn store_file(&self, file_id: String, file: Bytes) -> Result<(), util::PlainchantErr>;
    fn get_file(&self, file_id: String) -> Result<Bytes, util::PlainchantErr>;
    fn delete_file(&self, file_id: String) -> Result<(), util::PlainchantErr>;
}
