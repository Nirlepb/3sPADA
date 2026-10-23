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
