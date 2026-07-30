use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn temp_root(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    std::env::temp_dir().join(format!("klint-rs-{name}-{id}"))
}
