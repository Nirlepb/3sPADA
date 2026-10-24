mod common;
use common::{TempDir, transaction};
use merklelsm::Db;
use std::collections::BTreeMap;
#[test]
fn bounded_merge_matches_reference_and_skips_blocks() {
    let temp = TempDir::new("bounded");
    let mut db = Db::open(temp.db()).unwrap();
    let mut expected = BTreeMap::new();
    for i in 0..400 {
        let key = format!("key-{i:04}").into_bytes();
        let value = vec![b'x'; 100];
        db.put(&key, &value).unwrap();
        expected.insert(key, value);
    }
    db.flush().unwrap();
    for i in (0..400).step_by(3) {
        let key = format!("key-{i:04}").into_bytes();
        db.delete(&key).unwrap();
        expected.remove(&key);
    }
    db.flush().unwrap();
    db.put(b"key-0200", b"new").unwrap();
    expected.insert(b"key-0200".to_vec(), b"new".to_vec());
    db.put("é".as_bytes(), b"unicode").unwrap();
    expected.insert("é".as_bytes().to_vec(), b"unicode".to_vec());
    db.record(transaction("hidden")).unwrap();
    let (_, full) = db.scan_with_block_reads(None, None).unwrap();
    let (_, narrow) = db
        .scan_with_block_reads(Some(b"key-0200"), Some(b"key-0202"))
        .unwrap();
    assert!(narrow < full, "narrow {narrow}, full {full}");
    let bounds: Vec<Option<&[u8]>> = vec![
        None,
        Some(b""),
        Some(b"key-0000"),
        Some(b"key-0200"),
        Some(b"key-0202"),
        Some(b"zzz"),
        Some("é".as_bytes()),
        Some(&[255]),
    ];
    for &start in &bounds {
        for &end in &bounds {
            if start.zip(end).is_some_and(|(a, b)| a > b) {
                assert!(db.scan(start, end).is_err());
                continue;
            }
            let actual: Vec<_> = db
                .scan(start, end)
                .unwrap()
                .into_iter()
                .map(|e| (e.key, e.value))
                .collect();
            let reference: Vec<_> = expected
                .iter()
                .filter(|(k, _)| {
                    start.is_none_or(|s| k.as_slice() >= s) && end.is_none_or(|e| k.as_slice() < e)
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            assert_eq!(actual, reference);
        }
    }
    db.compact().unwrap();
    drop(db);
    let db = Db::open(temp.db()).unwrap();
    assert_eq!(db.scan(None, None).unwrap().len(), expected.len());
}

#[test]
fn binary_bounds_and_versions_survive_compaction_and_reopen() {
    let temp = TempDir::new("binary-bounds");
    let mut db = Db::open(temp.db()).unwrap();
    let keys = [vec![0], vec![0, 255], vec![128], vec![255], vec![255, 0]];
    for key in &keys {
        db.put(key, b"old").unwrap();
    }
    db.flush().unwrap();
    db.delete(&keys[1]).unwrap();
    db.put(&keys[2], b"disk-update").unwrap();
    db.flush().unwrap();
    db.put(&keys[2], b"memory-update").unwrap();
    db.put(&keys[3], b"").unwrap();
    db.record(transaction("not-a-mutable-key")).unwrap();
    for phase in 0..3 {
        let entries = db.scan(Some(&[128]), Some(&[255, 0])).unwrap();
        assert_eq!(
            entries
                .into_iter()
                .map(|e| (e.key, e.value))
                .collect::<Vec<_>>(),
            vec![(vec![128], b"memory-update".to_vec()), (vec![255], vec![])]
        );
        assert_eq!(db.scan(None, None).unwrap().len(), 4);
        let (empty, reads) = db
            .scan_with_block_reads(Some(&[128]), Some(&[128]))
            .unwrap();
        assert!(empty.is_empty());
        assert_eq!(reads, 0);
        match phase {
            0 => db.compact().unwrap(),
            1 => {
                drop(db);
                db = Db::open(temp.db()).unwrap();
            }
            _ => {}
        }
    }
}

#[test]
fn bounded_scan_authenticates_selected_blocks_after_open() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let temp = TempDir::new("scan-tamper");
    let mut db = Db::open(temp.db()).unwrap();
    for i in 0..100 {
        db.put(format!("key-{i:03}").as_bytes(), &[b'x'; 200])
            .unwrap();
    }
    db.flush().unwrap();
    let table = db.inspect_tables().unwrap().remove(0);
    assert!(table.blocks.len() > 2);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(temp.db().join(&table.file))
        .unwrap();
    let offset = table.blocks[0].offset + 4;
    file.seek(SeekFrom::Start(offset)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 1;
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();
    // The narrow query uses a different block; selecting the changed block fails.
    assert_eq!(db.scan(Some(b"key-099"), None).unwrap().len(), 1);
    assert!(matches!(
        db.scan(Some(b"key-000"), Some(b"key-001")),
        Err(merklelsm::Error::TamperDetected(_))
    ));
    assert!(db.scan(None, None).is_err());
}
