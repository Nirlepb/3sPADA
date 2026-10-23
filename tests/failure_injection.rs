#![cfg(feature = "fault-injection")]
mod common;
use common::TempDir;
use merklelsm::{Db, Error};
use std::process::Command;
#[test]
fn worker() {
    let Some(path) = std::env::var_os("MERKLELSM_FAULT_DB") else {
        return;
    };
    let stage = std::env::var("MERKLELSM_FAULT_STAGE").unwrap();
    let mut db = Db::open(path).unwrap();
    let result = if stage.starts_with("compact") {
        db.compact()
    } else {
        db.flush()
    };
    assert!(result.is_err());
    assert!(matches!(
        db.put(b"forbidden", b"value"),
        Err(Error::Poisoned)
    ));
    assert!(matches!(db.get(b"ack"), Err(Error::Poisoned)));
}
#[test]
fn publication_boundaries_recover_and_errors_poison() {
    for stage in [
        "flush-before-table",
        "flush-after-table",
        "flush-after-manifest",
        "compact-after-manifest",
    ] {
        for action in ["exit", "error"] {
            let temp = TempDir::new("fault");
            let mut db = Db::open(temp.db()).unwrap();
            db.put(b"ack", b"old").unwrap();
            db.flush().unwrap();
            db.put(b"ack", b"latest").unwrap();
            db.put(b"other", b"durable").unwrap();
            if stage.starts_with("compact") {
                db.flush().unwrap();
            }
            let sequence = db.stats().last_sequence;
            drop(db);
            let status = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "worker", "--nocapture"])
                .env("MERKLELSM_FAULT_DB", temp.db())
                .env("MERKLELSM_FAULT_STAGE", stage)
                .env("MERKLELSM_FAULT_ACTION", action)
                .status()
                .unwrap();
            if action == "exit" {
                assert_eq!(status.code(), Some(86));
            } else {
                assert!(status.success());
            }
            let mut db = Db::open(temp.db()).unwrap();
            assert_eq!(db.get(b"ack").unwrap(), Some(b"latest".to_vec()));
            assert_eq!(db.get(b"other").unwrap(), Some(b"durable".to_vec()));
            assert_eq!(db.get(b"forbidden").unwrap(), None);
            assert_eq!(db.stats().last_sequence, sequence);
            assert_eq!(db.put(b"next", b"ok").unwrap(), sequence + 1);
            db.flush().unwrap();
            db.verify().unwrap();
        }
    }
}
