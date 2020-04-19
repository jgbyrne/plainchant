use std::time::{SystemTime, UNIX_EPOCH, Duration};

pub fn timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
                     .unwrap_or(Duration::from_secs(0))
                     .as_secs()
}
