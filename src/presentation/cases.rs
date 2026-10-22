//! Small real database exercises for a viva. Isolated from presentation data.
use merklelsm::{Db, Error, Result, Transaction};
use std::{fs, io::Write, path::Path};

fn transaction(id: &str) -> Transaction {
    Transaction {
        id: id.into(),
        from: "Alice".into(),
        to: "Bob".into(),
        amount_paise: 1050,
        timestamp_ms: 1700000000000,
        note: "Live edge-case exercise".into(),
    }
}
fn check(name: &str, passed: bool) -> Result<()> {
    if !passed {
        return Err(Error::Corruption(format!(
            "edge-case exercise FAILED: {name}"
        )));
    }
    println!("  PASS: {name}");
    Ok(())
}
pub fn run(root: &Path) -> Result<()> {
    fs::create_dir(root)?;
    println!(
        "\nREAL EDGE-CASE EXERCISES\nSeparate database: {}",
        root.display()
    );
    let mut db = Db::open(root.join("db"))?;
    db.record(transaction("tx-001"))?;
    let before = db.stats().last_sequence;
    let duplicate = db.record(transaction("tx-001"));
    check(
        "duplicate ID tx-001 rejected before a new sequence/WAL write",
        matches!(duplicate, Err(Error::DuplicateTransaction(_)))
            && db.stats().last_sequence == before,
    )?;
    for amount in [0, -100] {
        let mut tx = transaction("invalid-amount");
        tx.amount_paise = amount;
        check(
            &format!("amount {amount} paise rejected"),
            matches!(db.record(tx), Err(Error::InvalidInput(_)))
                && db.stats().last_sequence == before,
        )?;
    }
    let mut tx = transaction("same-account");
    tx.to = tx.from.clone();
    check(
        "Alice -> Alice rejected",
        matches!(db.record(tx), Err(Error::InvalidInput(_))),
    )?;
    check(
        "blank transaction ID rejected",
        matches!(db.record(transaction(" ")), Err(Error::InvalidInput(_))),
    )?;
    check(
        "unknown transaction returns absence",
        db.get_transaction("missing")?.is_none(),
    )?;
    let mut tx = transaction("largest-amount");
    tx.amount_paise = i64::MAX;
    db.record(tx)?;
    check(
        "maximum positive i64 paise stored exactly",
        db.get_transaction("largest-amount")?
            .is_some_and(|tx| tx.amount_paise == i64::MAX),
    )?;
    db.put(b"empty", b"")?;
    check(
        "empty value is present, not absent",
        db.get(b"empty")? == Some(Vec::new()),
    )?;
    db.put(b"tx-001", b"mutable")?;
    db.delete(b"tx-001")?;
    check(
        "mutable namespace cannot overwrite/delete ledger receipt",
        db.get_transaction("tx-001")? == Some(transaction("tx-001")),
    )?;
    check(
        "empty mutable key rejected",
        matches!(db.put(b"", b"value"), Err(Error::InvalidInput(_))),
    )?;
    check(
        "reversed scan bounds rejected",
        matches!(db.scan(Some(b"z"), Some(b"a")), Err(Error::InvalidInput(_))),
    )?;
    db.put(b"old", b"previous-version")?;
    db.flush()?;
    db.delete(b"old")?;
    db.flush()?;
    check(
        "tombstone hides an older table value",
        db.get(b"old")?.is_none(),
    )?;
    db.compact()?;
    check(
        "full compaction does not resurrect a deleted key",
        db.get(b"old")?.is_none(),
    )?;
    let checkpoint = root.join("checkpoint.json");
    db.checkpoint(&checkpoint)?;
    db.verify_checkpoint(&checkpoint)?;
    check("flushed snapshot matches external checkpoint", true)?;
    db.put(b"new", b"later-write")?;
    check(
        "later legitimate write differs from an older exact snapshot",
        matches!(
            db.verify_checkpoint(&checkpoint),
            Err(Error::TamperDetected(_))
        ),
    )?;
    let recovery_path = root.join("recovery-db");
    {
        let mut writer = Db::open(&recovery_path)?;
        writer.put(b"pending", b"saved-in-WAL")?;
    }
    {
        let reader = Db::open(&recovery_path)?;
        check(
            "WAL-only write recovers without an SSTable",
            reader.stats().sstables == 0
                && reader.get(b"pending")? == Some(b"saved-in-WAL".to_vec()),
        )?;
    }
    fs::OpenOptions::new()
        .append(true)
        .open(recovery_path.join("WAL"))?
        .write_all(&[1, 2])?;
    {
        let reader = Db::open(&recovery_path)?;
        check(
            "two-byte torn WAL tail removed; complete write preserved",
            reader.stats().recovered_tail_bytes == 2 && reader.get(b"pending")?.is_some(),
        )?;
    }
    let crc_path = root.join("crc-db");
    {
        let mut writer = Db::open(&crc_path)?;
        writer.put(b"key", b"value")?;
    }
    let wal = crc_path.join("WAL");
    let mut bytes = fs::read(&wal)?;
    let last = bytes
        .last_mut()
        .ok_or_else(|| Error::Corruption("exercise WAL is empty".into()))?;
    *last ^= 1;
    fs::write(&wal, bytes)?;
    check(
        "complete record with a bad CRC is an ERROR, not a discarded tail",
        matches!(Db::open(&crc_path), Err(Error::Corruption(_))),
    )?;
    println!(
        "All 18 edge-case exercises passed using real storage operations.\nYour presentation database was not modified by these exercises. Artifacts retained for inspection."
    );
    Ok(())
}
