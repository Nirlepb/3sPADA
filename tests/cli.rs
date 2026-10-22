mod common;
use common::TempDir;
use std::process::{Command, Output};
fn run(temp: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_merklelsm"))
        .arg("--db")
        .arg(temp.db())
        .args(args)
        .output()
        .unwrap()
}
fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn complete_cli_demo_and_failure_exit_codes() {
    let temp = TempDir::new("cli");
    assert!(!run(&temp, &["get", "missing"]).status.success());
    assert!(!temp.db().exists());
    success(run(&temp, &["init"]));
    success(run(&temp, &["put", "hello", "world"]));
    assert_eq!(success(run(&temp, &["get", "hello"])).trim(), "\"world\"");
    let record = [
        "record",
        "--id",
        "tx-1",
        "--from",
        "alice",
        "--to",
        "bob",
        "--amount-paise",
        "1050",
        "--timestamp-ms",
        "1700000000000",
    ];
    success(run(&temp, &record));
    assert!(!run(&temp, &record).status.success());
    assert!(success(run(&temp, &["transaction", "tx-1"])).contains("1050"));
    success(run(&temp, &["flush"]));
    success(run(&temp, &["verify"]));
    let checkpoint = temp.path().join("checkpoint.json");
    let checkpoint = checkpoint.to_str().unwrap();
    success(run(&temp, &["checkpoint", checkpoint]));
    success(run(&temp, &["verify", "--checkpoint", checkpoint]));
    assert!(!run(&temp, &["tamper"]).status.success());
    success(run(&temp, &["tamper", "--allow-demo-tamper"]));
    let bad = run(&temp, &["verify"]);
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("data block 0"));
}

#[test]
fn cli_scan_delete_and_compact() {
    let temp = TempDir::new("cli-scan");
    success(run(&temp, &["put", "a", "one"]));
    success(run(&temp, &["flush"]));
    success(run(&temp, &["put", "b", "two"]));
    success(run(&temp, &["delete", "a"]));
    assert_eq!(success(run(&temp, &["get", "a"])).trim(), "null");
    let scan = success(run(&temp, &["scan", "--start", "b", "--end", "c"]));
    assert!(scan.contains("two"));
    assert!(!scan.contains("one"));
    success(run(&temp, &["compact"]));
    success(run(&temp, &["verify"]));
    assert!(success(run(&temp, &["stats"])).contains("\"sstables\": 1"));
}
