#![allow(dead_code)]
use merklelsm::Transaction;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct TempDir(PathBuf);
impl TempDir {
    pub fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "merklelsm-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create test directory");
        Self(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn db(&self) -> PathBuf {
        self.0.join("db")
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub fn transaction(id: &str) -> Transaction {
    Transaction {
        id: id.into(),
        from: "alice".into(),
        to: "bob".into(),
        amount_paise: 1050,
        timestamp_ms: 1700000000000,
        note: "test payment".into(),
    }
}
