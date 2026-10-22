//! Kill a real process after it signals that all writes have been synced.
mod common;
use common::TempDir;
use merklelsm::Db;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn crash_worker() {
    let Some(path) = std::env::var_os("MERKLELSM_CRASH_WORKER_DB") else {
        return;
    };
    let mut db = Db::open(path).unwrap();
    for i in 0u32..30 {
        db.put(&i.to_be_bytes(), b"acknowledged").unwrap();
    }
    println!("ALL_WRITES_SYNCED");
    std::io::stdout().flush().unwrap();
    // Keep the file handles alive until the parent kills us; no Drop runs.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

#[test]
fn killed_process_recovers_acknowledged_writes_and_releases_lock() {
    let temp = TempDir::new("kill");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_worker", "--nocapture"])
        .env("MERKLELSM_CRASH_WORKER_DB", temp.db())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let reader = BufReader::new(child.stdout.take().unwrap());
    let mut ready = false;
    for line in reader.lines() {
        if line.unwrap().contains("ALL_WRITES_SYNCED") {
            ready = true;
            break;
        }
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(ready, "worker failed before durable writes");
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.stats().sstables, 0);
    for i in 0u32..30 {
        assert_eq!(
            db.get(&i.to_be_bytes()).unwrap(),
            Some(b"acknowledged".to_vec())
        );
    }
    db.verify().unwrap();
}
