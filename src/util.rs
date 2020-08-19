use std::time::{SystemTime, UNIX_EPOCH, Duration};
use chrono::Utc;

#[derive(Debug)]
pub enum ErrOrigin {
    Database,
    FileRack,
    Template,
    Web(u16),
}

#[derive(Debug)]
pub struct PlainchantErr {
    pub origin: ErrOrigin,
    pub msg: String,
}


pub fn timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
                     .unwrap_or(Duration::from_secs(0))
                     .as_secs()
}

pub fn fmt_time(ts: u64) -> String {
    Utc::now().to_rfc3339()
}
