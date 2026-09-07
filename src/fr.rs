use crate::util;

use bytes::Bytes;

pub fn static_err_with_code(msg: &'static str, code: u16) -> util::PlainchantErr {
    util::PlainchantErr {
        origin: util::ErrOrigin::FileRack,
        code,
        msg:    String::from(msg),
    }
}

pub fn static_err(msg: &'static str) -> util::PlainchantErr {
    static_err_with_code(msg, 500)
}

pub trait FileRack: Sync + Send + 'static {
    fn store_file(&self, file_id: &str, file: Bytes) -> Result<(), util::PlainchantErr>;
    fn get_file(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr>;
    fn get_file_thumbnail(&self, file_id: &str) -> Result<Bytes, util::PlainchantErr>;
    fn delete_file(&self, file_id: &str) -> Result<(), util::PlainchantErr>;
}
