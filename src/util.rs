use lazy_static::lazy_static;
use regex::Regex;
use std::process::exit;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

lazy_static! {
    // Capture possible URLs
    pub static ref URL: Regex = Regex::new(r"(http|https)://([A-Za-z0-9\-_~:/?.#@!$&'()*+,;%=]+)").unwrap();
}

#[derive(Debug)]
pub enum ErrOrigin {
    Database,
    FileRack,
    Actions,
    Template,
    Web(u16),
}

#[derive(Debug)]
pub struct PlainchantErr {
    pub origin: ErrOrigin,
    pub msg:    String,
}

impl PlainchantErr {
    pub fn die(&self) -> ! {
        eprintln!("Fatal Error - {:?} - {}", &self.origin, &self.msg);
        exit(1);
    }
}

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

macro_rules! unwrap_or_return {
    ( $test:expr, $ret:expr ) => {
        match $test {
            Ok(val) => val,
            Err(_) => {
                return $ret;
            },
        }
    };
}

pub(crate) use unwrap_or_return;
