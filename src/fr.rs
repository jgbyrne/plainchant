use crate::site;
use crate::util;

use std::io;

pub trait FileRack {
    fn store_file(&self, file_id: String, file: io::Bytes)Result<(), util::PlanchantError>;
    fn get_file(&self, file_id: String) -> io::Bytes;
    fn delete_file(&self, file_id: String) -> Result<(), util::PlanchantError>;
}
