# Local validation record

Validated on 22 October 2026 in `/home/tharunn/Desktop/3sPADA`.

## Environment

- Linux 7.0.0-31-generic, x86_64.
- `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.
- `cargo 1.99.1 (5f94df478 2026-08-27)`.
- Dependencies fetched from crates.io and resolved in `Cargo.lock`; later checks used cached dependencies with `--offline`.
- Declared minimum Rust version: 1.89. That minimum was not separately installed and validated.

## Results

| Check | Result |
| --- | --- |
| `cargo check --offline` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --offline --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test --locked --offline --all-targets` | 33 test functions passed; 0 failed; 0 ignored |
| `cargo test --locked --offline --doc` | Passed; no executable doctests defined |
| `cargo doc --locked --offline --no-deps` | API documentation generated successfully |
| `bash -n scripts/demo.sh` | Shell syntax check passed |
| `bash scripts/demo.sh` | Passed, including expected duplicate rejection and block-0 corruption failure |
| Library example | Recorded, flushed, verified, and retrieved `example-001` |
| Release timing example | Completed 1,000 synced writes, reads, flush, compaction, and verification |
| `git diff --check` | Passed for tracked changes |
| Original plan comparison | Preserved copy is byte-for-byte identical to the supplied `Plan` |

The 33 functions comprise 3 module unit tests, 2 CLI tests, 2 process-crash tests, 7 integrity tests, 7 persistence tests, 7 recovery tests, and 5 interactive presentation tests. One process-crash function is the child-worker helper; its parent exercises it in a separate process, while the ordinary invocation returns immediately. There are no ignored placeholder cases.

The integrity tests check that malformed generations or reordered manifest tables cannot cause reuse of an active table filename. The five presentation tests send real typed input through the executable and cover actual memory/disk traces, WAL reopen, inspected Merkle roots, malformed/negative/zero inputs, duplicate rejection, checkpoints, tampering with a copy, automatic flush, EOF halfway through a form, and all 18 isolated edge-case exercises. The complete 33-test suite, formatting check, Clippy, doctest check, original demo script, and command help passed after presentation mode was added. Local documentation links also resolve.

## Example runs

```bash
cargo run --locked --offline --example basic -- target/validation-basic-db
cargo run --locked --offline --release --example benchmark -- target/validation-benchmark-db 1000
```

The basic example returned Alice-to-Bob transaction `example-001` for 1,050 paise. Validation databases remain under ignored `target/` for inspection.

| Timing operation | Observed elapsed time |
| --- | --- |
| 1,000 writes, each syncing its WAL record | 1.209087793 s |
| Final flush | 5.975185 ms |
| 1,000 reads | 10.9852 ms |
| Full compaction | 8.56705 ms |

These are one local run, not portable performance claims. Write timing includes automatic flushes. OS cache, storage, sync behavior, and build mode affect results.

## Validation limits

The process termination test recovers acknowledged writes without destructors running. Recovery tests reconstruct publication failures and force a real manifest-write I/O failure. Physical power loss, every filesystem, disk-full behavior, extensive adversarial fuzzing, and Windows have not been exhaustively tested.

GitHub CI is configured but has not been run remotely by this task. No commit, push, pull request, issue posting, or merge was performed. Source and documentation remain local working-tree changes for the project owner to review.
