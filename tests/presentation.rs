mod common;
use common::TempDir;
use merklelsm::Db;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn present(temp: &TempDir, input: &str, extra: &[&str]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_merklelsm"))
        .arg("--db")
        .arg(temp.db())
        .args(extra)
        .arg("present")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn live_inputs_show_real_memory_disk_read_paths_and_recovery() {
    let temp = TempDir::new("present-live");
    let out = present(
        &temp,
        "1\ntx-live\nAlice\nBob\n1050\n1700000000000\nLunch\n3\n2\ntx-live\n12\n2\ntx-live\n4\n5\n2\ntx-live\n2\nmissing\n0\n",
        &[],
    );
    for expected in [
        "RECORDED at sequence 1",
        "WAL appended length + entry + CRC32",
        "ACTUAL SKIP-LIST LINKS",
        "Memtable HIT",
        "recovered memtable entries: 1",
        "SPARSE INDEX AND DATA BLOCKS",
        "MERKLE LEVELS",
        "Bloom filter says MAYBE PRESENT",
        "Sparse index binary search selected data block",
        "NOT FOUND: no transaction",
    ] {
        assert!(out.contains(expected), "missing {expected}:\n{out}");
    }
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(
        db.get_transaction("tx-live").unwrap().unwrap().amount_paise,
        1050
    );
    let inspection = db.inspect_tables().unwrap();
    assert_eq!(
        inspection[0].merkle_levels.last().unwrap(),
        &vec![inspection[0].root.clone()]
    );
}

#[test]
fn invalid_input_and_duplicate_rejection_keep_the_menu_usable() {
    let temp = TempDir::new("present-errors");
    let out = present(
        &temp,
        "99\n1\nbad-number\nAlice\nBob\nabc\n1\nbase\nAlice\nBob\n1050\n\n\n1\nbase\nAlice\nBob\n1050\n\n\n1\nzero\nAlice\nBob\n0\n\n\n1\nnegative\nAlice\nBob\n-1\n\n\n17\n0\n",
        &[],
    );
    assert!(out.contains("Choose one of the displayed numbers"));
    assert!(out.contains("enter a whole number"));
    assert!(out.contains("Duplicate ID found; rejected before any WAL append"));
    assert!(out.contains("positive integer paise"));
    assert!(out.contains("Sequence before: 1; after: 1"));
    assert_eq!(
        Db::open(temp.db()).unwrap().transactions().unwrap().len(),
        1
    );
}

#[test]
fn checkpoint_and_copy_tamper_demo_preserve_the_original_database() {
    let temp = TempDir::new("present-tamper");
    let checkpoint = temp.path().join("trusted.json");
    let input = format!(
        "7\nnote\nold\n4\n7\nnote\nnew\n7\ntemporary\nremove\n9\ntemporary\n10\n\n\n11\n15\n{}\n16\n{}\n14\n\n\n6\n8\nnote\n0\n",
        checkpoint.display(),
        checkpoint.display()
    );
    let out = present(&temp, &input, &[]);
    for expected in [
        "Heap merge of all",
        "Tombstone written",
        "Checkpoint created",
        "matches the exact trusted checkpoint",
        "PASS: tamper detected",
        "data block 0",
        "Expected block leaf",
        "Changed block leaf",
        "Original database verification: PASS",
        "FOUND: \"new\"",
    ] {
        assert!(out.contains(expected), "missing {expected}:\n{out}");
    }
    let db = Db::open(temp.db()).unwrap();
    db.verify_checkpoint(checkpoint).unwrap();
    assert!(db.get(b"temporary").unwrap().is_none());
}

#[test]
fn edge_case_exercises_run_in_isolation_and_eof_exits_cleanly() {
    let temp = TempDir::new("present-cases");
    let out = present(&temp, "13\n17\n", &[]);
    assert!(out.contains("All 18 edge-case exercises passed"), "{out}");
    assert!(out.contains("complete record with a bad CRC is an ERROR"));
    assert_eq!(Db::open(temp.db()).unwrap().stats().last_sequence, 0);
    let out = present(&temp, "1\nunfinished\n", &[]);
    assert!(out.contains("Session closed"));
    assert_eq!(Db::open(temp.db()).unwrap().stats().last_sequence, 0);
}

#[test]
fn automatic_flush_is_reported_only_after_the_real_threshold_is_reached() {
    let temp = TempDir::new("present-auto");
    let out = present(&temp, "7\nkey\nvalue\n3\n0\n", &["--memtable-bytes", "1"]);
    assert!(out.contains("Memtable threshold 1 reached; automatic flush"));
    assert!(out.contains("WAL truncated and synced AFTER"));
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.stats().sstables, 1);
    assert_eq!(db.stats().memtable_entries, 0);
}
