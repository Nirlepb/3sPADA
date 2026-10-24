mod common;
use common::{TempDir, transaction};
use merklelsm::{Db, manifest::Manifest, merkle};
#[test]
fn paths_validate_shape_and_odd_duplication() {
    for count in 1..18 {
        let bytes: Vec<_> = (0..count).map(|i| vec![i as u8]).collect();
        let leaves: Vec<_> = bytes.iter().map(|b| merkle::hash_leaf(b)).collect();
        let root = merkle::root(&leaves);
        for (index, value) in bytes.iter().enumerate() {
            let proof = merkle::prove(&leaves, index).unwrap();
            assert!(proof.verify(value, &root));
            assert!(!proof.verify(b"changed", &root));
            assert!(!proof.verify(value, &[0; 32]));
            let mut bad = proof.clone();
            bad.leaf_index = count;
            assert!(!bad.verify(value, &root));
            let mut bad = proof.clone();
            bad.leaf_count = 0;
            assert!(!bad.verify(value, &root));
            let mut bad = proof.clone();
            bad.siblings.push([0; 32]);
            assert!(!bad.verify(value, &root));
            if !proof.siblings.is_empty() {
                let mut bad = proof.clone();
                bad.siblings[0][0] ^= 1;
                assert!(!bad.verify(value, &root));
                let mut bad = proof.clone();
                bad.siblings.pop();
                assert!(!bad.verify(value, &root));
            }
        }
    }
}
#[test]
fn transaction_proofs_are_bound_to_receipt_and_checkpoint() {
    let temp = TempDir::new("proof");
    let mut db = Db::open(temp.db()).unwrap();
    for i in 0..90 {
        db.record(transaction(&format!("tx-{i:03}"))).unwrap();
    }
    assert!(db.transaction_proof("tx-000").is_err());
    let checkpoint_path = temp.path().join("trusted.json");
    db.checkpoint(&checkpoint_path).unwrap();
    let checkpoint = Manifest::read(&checkpoint_path).unwrap();
    assert!(db.stats().data_blocks > 2);
    for id in ["tx-000", "tx-045", "tx-089"] {
        let proof = db.transaction_proof(id).unwrap();
        let json = serde_json::to_vec(&proof).unwrap();
        let proof: merklelsm::proof::TransactionProof = serde_json::from_slice(&json).unwrap();
        assert_eq!(proof.verify(id, &checkpoint).unwrap().id, id);
        assert!(proof.verify("missing", &checkpoint).is_err());
        let mut bad = proof.clone();
        bad.framed_block[5] ^= 1;
        assert!(bad.verify(id, &checkpoint).is_err());
        let mut bad = proof.clone();
        bad.table = "unknown.sst".into();
        assert!(bad.verify(id, &checkpoint).is_err());
        let mut bad = proof.clone();
        bad.path.leaf_index = 0;
        assert!(bad.verify(id, &checkpoint).is_err());
        let mut wrong = checkpoint.clone();
        wrong.tables[0].root[0] ^= 1;
        assert!(proof.verify(id, &wrong).is_err());
    }
    assert!(db.transaction_proof("missing").is_err());
    db.compact().unwrap();
    assert!(
        db.transaction_proof("tx-000")
            .unwrap()
            .verify("tx-000", &checkpoint)
            .is_err()
    );
    let compacted_path = temp.path().join("compacted-checkpoint.json");
    db.checkpoint(&compacted_path).unwrap();
    let compacted = Manifest::read(&compacted_path).unwrap();
    drop(db);
    let db = Db::open(temp.db()).unwrap();
    for id in ["tx-000", "tx-045", "tx-089"] {
        assert_eq!(
            db.transaction_proof(id)
                .unwrap()
                .verify(id, &compacted)
                .unwrap(),
            transaction(id)
        );
    }
}

#[test]
fn cli_exports_and_verifies_without_opening_database() {
    use std::process::Command;
    let temp = TempDir::new("proof-cli");
    let mut db = Db::open(temp.db()).unwrap();
    db.record(transaction("tx-cli")).unwrap();
    let checkpoint = temp.path().join("checkpoint.json");
    db.checkpoint(&checkpoint).unwrap();
    drop(db);
    let proof = temp.path().join("proof.json");
    let binary = env!("CARGO_BIN_EXE_merklelsm");
    let result = Command::new(binary)
        .arg("--db")
        .arg(temp.db())
        .args(["prove", "tx-cli"])
        .arg(&proof)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let absent_db = temp.path().join("must-not-create");
    let result = Command::new(binary)
        .arg("--db")
        .arg(&absent_db)
        .args(["verify-proof", "tx-cli"])
        .arg(&proof)
        .arg(&checkpoint)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!absent_db.exists());
    let mut bad: merklelsm::proof::TransactionProof =
        serde_json::from_slice(&std::fs::read(&proof).unwrap()).unwrap();
    bad.framed_block[5] ^= 1;
    std::fs::write(&proof, serde_json::to_vec(&bad).unwrap()).unwrap();
    let result = Command::new(binary)
        .args(["verify-proof", "tx-cli"])
        .arg(&proof)
        .arg(&checkpoint)
        .output()
        .unwrap();
    assert!(!result.status.success());
}
