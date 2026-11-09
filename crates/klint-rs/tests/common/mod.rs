use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn temp_root(name: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("klint-rs-{name}-{}-{id}", std::process::id()))
}
