mod common;
use common::{TempDir, transaction};
use merklelsm::{Db, Error, manifest::Manifest, sstable::SsTable};
use std::fs;

#[test]
fn tampering_reports_exact_data_block() {
    let temp = TempDir::new("tamper");
    {
        let mut db = Db::open(temp.db()).unwrap();
        for i in 0..100 {
            db.put(format!("{i:03}").as_bytes(), &[42; 100]).unwrap();
        }
        db.flush().unwrap();
    }
    let manifest = Manifest::load(&temp.db()).unwrap();
    let meta = &manifest.tables[0];
    let path = temp.db().join(&meta.file);
    let table = SsTable::open(&path, meta).unwrap();
    assert!(table.block_count() > 1);
    let offset = table.block_payload_offset(1).unwrap() as usize;
    let mut bytes = fs::read(&path).unwrap();
    bytes[offset] ^= 1;
    fs::write(&path, bytes).unwrap();
    let error = match Db::open(temp.db()) {
        Ok(_) => panic!("tamper accepted"),
        Err(e) => e,
    };
    assert!(error.to_string().contains("data block 1"), "{error}");
}

#[test]
fn every_byte_in_small_table_is_authenticated() {
    let temp = TempDir::new("all-bytes");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"value").unwrap();
        db.flush().unwrap();
    }
    let manifest = Manifest::load(&temp.db()).unwrap();
    let meta = &manifest.tables[0];
    let path = temp.db().join(&meta.file);
    let original = fs::read(&path).unwrap();
    for offset in 0..original.len() {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert!(
            SsTable::open(&path, meta).is_err(),
            "unprotected byte {offset}"
        );
    }
    fs::write(&path, &original).unwrap();
    SsTable::open(&path, meta).unwrap();
}

#[test]
fn external_checkpoint_detects_manifest_and_table_replacement() {
    let temp = TempDir::new("checkpoint");
    let checkpoint = temp.path().join("checkpoint.json");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.record(transaction("tx")).unwrap();
        db.checkpoint(&checkpoint).unwrap();
        db.verify_checkpoint(&checkpoint).unwrap();
        assert!(db.checkpoint(&checkpoint).is_err());
    }
    let other = temp.path().join("replacement");
    {
        let mut db = Db::open(&other).unwrap();
        let mut tx = transaction("tx");
        tx.amount_paise = 9000;
        db.record(tx).unwrap();
        db.flush().unwrap();
    }
    let replacement = Manifest::load(&other).unwrap();
    for table in &replacement.tables {
        fs::copy(other.join(&table.file), temp.db().join(&table.file)).unwrap();
    }
    fs::copy(other.join("MANIFEST.json"), temp.db().join("MANIFEST.json")).unwrap();
    let db = Db::open(temp.db()).unwrap();
    db.verify().unwrap();
    assert!(matches!(
        db.verify_checkpoint(checkpoint),
        Err(Error::TamperDetected(_))
    ));
}

#[test]
fn checkpoint_requires_exact_snapshot_and_external_location() {
    let temp = TempDir::new("snapshot");
    let mut db = Db::open(temp.db()).unwrap();
    assert!(db.checkpoint(temp.db().join("inside.json")).is_err());
    let checkpoint = temp.path().join("snapshot.json");
    db.checkpoint(&checkpoint).unwrap();
    db.verify_checkpoint(&checkpoint).unwrap();
    db.put(b"later", b"write").unwrap();
    assert!(db.verify_checkpoint(&checkpoint).is_err());
    db.flush().unwrap();
    assert!(db.verify_checkpoint(&checkpoint).is_err());
}

#[test]
fn reader_detects_block_changes_after_open() {
    let temp = TempDir::new("live-tamper");
    let mut db = Db::open(temp.db()).unwrap();
    db.put(b"a", b"value").unwrap();
    db.flush().unwrap();
    let manifest = Manifest::load(&temp.db()).unwrap();
    let path = temp.db().join(&manifest.tables[0].file);
    let mut bytes = fs::read(&path).unwrap();
    bytes[36] ^= 1;
    fs::write(path, bytes).unwrap();
    assert!(matches!(db.get(b"a"), Err(Error::TamperDetected(_))));
    assert!(db.verify().is_err());
}

#[test]
fn manifest_rejects_path_traversal() {
    let temp = TempDir::new("path-traversal");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"v").unwrap();
        db.flush().unwrap();
    }
    let mut manifest = Manifest::load(&temp.db()).unwrap();
    manifest.tables[0].file = "../outside".into();
    fs::write(
        temp.db().join("MANIFEST.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(Db::open(temp.db()).is_err());
}

#[test]
fn malformed_manifest_generation_cannot_reuse_an_active_table_name() {
    let temp = TempDir::new("bad-generation");
    {
        let mut db = Db::open(temp.db()).unwrap();
        db.put(b"a", b"one").unwrap();
        db.flush().unwrap();
        db.put(b"b", b"two").unwrap();
        db.flush().unwrap();
    }
    let manifest = Manifest::load(&temp.db()).unwrap();
    let mut wrong = manifest.clone();
    wrong.generation = 1;
    fs::write(
        temp.db().join("MANIFEST.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert!(Db::open(temp.db()).is_err());
    let mut wrong = manifest;
    wrong.tables.reverse();
    fs::write(
        temp.db().join("MANIFEST.json"),
        serde_json::to_vec(&wrong).unwrap(),
    )
    .unwrap();
    assert!(Db::open(temp.db()).is_err());
}
