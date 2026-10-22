mod common;
use common::TempDir;
use merklelsm::{Db, Entry, Error, Kind, manifest::Manifest, wal::Wal};
use std::fs;

fn entry(seq: u64) -> Entry {
    Entry {
        key: format!("k:{seq}").into_bytes(),
        value: vec![seq as u8; 10],
        seq,
        kind: Kind::Put,
    }
}

#[test]
fn every_wal_truncation_recovers_only_complete_records_and_allows_append() {
    let temp = TempDir::new("wal-prefix");
    let source = temp.path().join("source");
    let mut boundaries = vec![0];
    {
        let (mut wal, _) = Wal::open(&source).unwrap();
        for seq in 1..=3 {
            wal.append(&entry(seq)).unwrap();
            boundaries.push(fs::metadata(&source).unwrap().len() as usize);
        }
    }
    let bytes = fs::read(source).unwrap();
    let target = temp.path().join("target");
    for cut in 0..=bytes.len() {
        fs::write(&target, &bytes[..cut]).unwrap();
        let (mut wal, replay) = Wal::open(&target).unwrap();
        let complete = boundaries.iter().skip(1).filter(|&&b| b <= cut).count();
        assert_eq!(replay.entries.len(), complete, "cut {cut}");
        assert_eq!(replay.valid_bytes as usize, boundaries[complete]);
        assert_eq!(
            replay.discarded_tail_bytes as usize,
            cut - boundaries[complete]
        );
        wal.append(&entry(complete as u64 + 1)).unwrap();
        drop(wal);
        assert_eq!(Wal::replay(&target).unwrap().entries.len(), complete + 1);
    }
}

#[test]
fn complete_crc_failure_is_never_treated_as_a_torn_tail() {
    let temp = TempDir::new("crc");
    let path = temp.path().join("WAL");
    {
        let (mut wal, _) = Wal::open(&path).unwrap();
        wal.append(&entry(1)).unwrap();
    }
    let mut bytes = fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    fs::write(&path, &bytes).unwrap();
    assert!(matches!(Wal::open(&path), Err(Error::Corruption(_))));
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn crash_after_manifest_commit_before_wal_reset_is_idempotent() {
    let temp = TempDir::new("manifest-wal");
    let old_wal;
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"one").unwrap();
        old_wal = fs::read(temp.db().join("WAL")).unwrap();
        db.flush().unwrap();
    }
    fs::write(temp.db().join("WAL"), old_wal).unwrap();
    {
        let mut db = Db::open(temp.db()).unwrap();
        assert_eq!(db.get(b"a").unwrap(), Some(b"one".to_vec()));
        assert_eq!(db.stats().memtable_entries, 0);
        assert_eq!(db.put(b"b", b"two").unwrap(), 2);
    }
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.get(b"b").unwrap(), Some(b"two".to_vec()));
}

#[test]
fn crash_before_manifest_commit_ignores_orphan_table_and_replays_wal() {
    let temp = TempDir::new("orphan");
    let old_manifest;
    let old_wal;
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"one").unwrap();
        old_manifest = fs::read(temp.db().join("MANIFEST.json")).unwrap();
        old_wal = fs::read(temp.db().join("WAL")).unwrap();
        db.flush().unwrap();
    }
    fs::write(temp.db().join("MANIFEST.json"), old_manifest).unwrap();
    fs::write(temp.db().join("WAL"), old_wal).unwrap();
    fs::write(temp.db().join(".MANIFEST.json.tmp"), b"half-written").unwrap();
    let mut db = Db::open(temp.db()).unwrap();
    assert_eq!(db.stats().sstables, 0);
    assert_eq!(db.get(b"a").unwrap(), Some(b"one".to_vec()));
    db.flush().unwrap();
    db.verify().unwrap();
}

#[test]
fn missing_manifest_or_table_fails_closed() {
    let temp = TempDir::new("missing");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"one").unwrap();
        db.flush().unwrap();
    }
    let manifest = Manifest::load(&temp.db()).unwrap();
    fs::remove_file(temp.db().join(&manifest.tables[0].file)).unwrap();
    assert!(Db::open(temp.db()).is_err());
    fs::remove_file(temp.db().join("MANIFEST.json")).unwrap();
    assert!(matches!(Db::open(temp.db()), Err(Error::Corruption(_))));
}

#[test]
fn db_reports_and_removes_torn_tail_before_next_write() {
    use std::io::Write;
    let temp = TempDir::new("db-tail");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"one").unwrap();
    }
    fs::OpenOptions::new()
        .append(true)
        .open(temp.db().join("WAL"))
        .unwrap()
        .write_all(&[1, 2])
        .unwrap();
    {
        let mut db = Db::open(temp.db()).unwrap();
        assert_eq!(db.stats().recovered_tail_bytes, 2);
        db.put(b"b", b"two").unwrap();
    }
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.get(b"b").unwrap(), Some(b"two".to_vec()));
    db.verify().unwrap();
}

#[test]
fn failed_manifest_publication_preserves_wal_and_requires_reopen() {
    let temp = TempDir::new("failed-flush");
    let mut db = Db::open(temp.db()).unwrap();
    db.put(b"a", b"saved").unwrap();
    let wal = fs::read(temp.db().join("WAL")).unwrap();
    // A directory at the temporary manifest path forces a real I/O error.
    fs::create_dir(temp.db().join(".MANIFEST.json.tmp")).unwrap();
    assert!(db.flush().is_err());
    assert!(matches!(db.put(b"b", b"no"), Err(Error::Poisoned)));
    assert_eq!(fs::read(temp.db().join("WAL")).unwrap(), wal);
    assert!(Manifest::load(&temp.db()).unwrap().tables.is_empty());
    drop(db);
    fs::remove_dir(temp.db().join(".MANIFEST.json.tmp")).unwrap();
    let mut db = Db::open(temp.db()).unwrap();
    assert_eq!(db.get(b"a").unwrap(), Some(b"saved".to_vec()));
    db.flush().unwrap();
    db.verify().unwrap();
}
