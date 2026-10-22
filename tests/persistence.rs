mod common;
use common::{TempDir, transaction};
use merklelsm::{Db, Error, Options};

#[test]
fn writes_survive_reopen_flush_and_compaction() {
    let temp = TempDir::new("persistence");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"b", b"old").unwrap();
        db.put(b"a", b"first").unwrap();
        db.record(transaction("tx-1")).unwrap();
        assert_eq!(db.stats().sstables, 0); // Exit with only a WAL, no flush.
    }
    {
        let mut db = Db::open(temp.db()).unwrap();
        assert_eq!(db.get(b"b").unwrap(), Some(b"old".to_vec()));
        db.flush().unwrap();
        db.put(b"b", b"new").unwrap();
        db.delete(b"a").unwrap();
        db.flush().unwrap();
        assert_eq!(db.stats().sstables, 2);
        assert_eq!(db.get(b"a").unwrap(), None);
        assert_eq!(db.get(b"b").unwrap(), Some(b"new".to_vec()));
        db.compact().unwrap();
        assert_eq!(db.stats().sstables, 1);
        db.verify().unwrap();
    }
    let mut db = Db::open(temp.db()).unwrap();
    assert_eq!(db.get(b"a").unwrap(), None);
    assert_eq!(db.get(b"b").unwrap(), Some(b"new".to_vec()));
    assert_eq!(
        db.get_transaction("tx-1").unwrap(),
        Some(transaction("tx-1"))
    );
    assert!(matches!(
        db.record(transaction("tx-1")),
        Err(Error::DuplicateTransaction(_))
    ));
    assert_eq!(db.scan(None, None).unwrap().len(), 1);
}

#[test]
fn ledger_cannot_be_overwritten_through_demo_api() {
    let temp = TempDir::new("namespaces");
    let mut db = Db::open(temp.db()).unwrap();
    db.record(transaction("same")).unwrap();
    assert!(matches!(
        db.record(transaction("same")),
        Err(Error::DuplicateTransaction(_))
    ));
    db.put(b"same", b"mutable").unwrap();
    db.put(b"t:same", b"also mutable").unwrap();
    db.delete(b"same").unwrap();
    assert_eq!(
        db.get_transaction("same").unwrap(),
        Some(transaction("same"))
    );
    assert_eq!(db.transactions().unwrap().len(), 1);
    let before = db.stats().last_sequence;
    let mut invalid = transaction("bad");
    invalid.amount_paise = 0;
    assert!(db.record(invalid).is_err());
    let mut invalid = transaction("bad");
    invalid.amount_paise = -1;
    assert!(db.record(invalid).is_err());
    assert!(db.put(b"", b"value").is_err());
    assert!(db.put(b"huge", &vec![0; 1_048_577]).is_err());
    assert_eq!(db.stats().last_sequence, before);
}

#[test]
fn automatic_flush_range_scan_and_empty_values() {
    let temp = TempDir::new("scan");
    let mut db = Db::open_with_options(temp.db(), Options { memtable_bytes: 80 }).unwrap();
    for key in [b"d", b"b", b"a", b"c"] {
        db.put(key, b"").unwrap();
    }
    assert!(db.stats().sstables > 0);
    assert_eq!(db.get(b"a").unwrap(), Some(vec![]));
    let keys: Vec<_> = db
        .scan(Some(b"b"), Some(b"d"))
        .unwrap()
        .into_iter()
        .map(|e| e.key)
        .collect();
    assert_eq!(keys, vec![b"b".to_vec(), b"c".to_vec()]);
    assert!(db.scan(Some(b"z"), Some(b"a")).is_err());
    assert!(db.scan(Some(b"b"), Some(b"b")).unwrap().is_empty());
}

#[test]
fn single_writer_lock_is_released_on_close() {
    let temp = TempDir::new("lock");
    let first = Db::open(temp.db()).unwrap();
    assert!(matches!(Db::open(temp.db()), Err(Error::Locked(_))));
    drop(first);
    Db::open(temp.db()).unwrap();
}

#[test]
fn all_deleted_compaction_and_sequence_survive_restart() {
    let temp = TempDir::new("empty-compact");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"x").unwrap();
        db.flush().unwrap();
        db.delete(b"a").unwrap();
        db.compact().unwrap();
        assert_eq!(db.stats().sstables, 0);
    }
    let mut db = Db::open(temp.db()).unwrap();
    assert_eq!(db.put(b"b", b"y").unwrap(), 3);
    db.verify().unwrap();
}

#[test]
fn large_record_spans_target_block_size() {
    let temp = TempDir::new("large");
    let value = vec![42; 100_000];
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"large", &value).unwrap();
        db.flush().unwrap();
    }
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.get(b"large").unwrap(), Some(value));
}

#[test]
fn randomized_history_matches_reference_map() {
    use std::collections::BTreeMap;
    let temp = TempDir::new("model");
    let mut expected = BTreeMap::new();
    let mut db = Db::open_with_options(
        temp.db(),
        Options {
            memtable_bytes: 1024,
        },
    )
    .unwrap();
    let mut random = 12345u64;
    for step in 0u32..200 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let key = format!("key-{:02}", random % 23).into_bytes();
        if random.is_multiple_of(4) {
            db.delete(&key).unwrap();
            expected.remove(&key);
        } else {
            let value = step.to_le_bytes().to_vec();
            db.put(&key, &value).unwrap();
            expected.insert(key, value);
        }
        if step % 37 == 0 {
            db.flush().unwrap();
        }
        if step % 61 == 0 {
            db.compact().unwrap();
        }
        if step % 43 == 0 {
            drop(db);
            db = Db::open(temp.db()).unwrap();
        }
        let actual: BTreeMap<_, _> = db
            .scan(None, None)
            .unwrap()
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect();
        assert_eq!(actual, expected, "step {step}");
    }
    db.verify().unwrap();
}
