# Testing and final debugging

## Reproduce the automated checks

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
cargo test --locked --doc
bash scripts/demo.sh
```

The suite uses isolated temporary directories and no external database service. `tests/common/mod.rs` removes its own directories when tests finish. The crash test starts a child process, waits for a durable-write signal, kills it, and reopens its database. Its helper test returns immediately during ordinary standalone execution.

## Coverage map

| Location | Evidence |
| --- | --- |
| `src/memtable/mod.rs` | Reverse-order insertion, ordered iteration, newer replacement, tombstone, size accounting, missing key |
| `src/bloom.rs` | 1,000 inserted keys have no false negatives; empty filter rejects an absent key |
| `src/merkle/mod.rs` | Known SHA-256 vector, domain separation, empty tree, odd-leaf duplication |
| `tests/persistence.rs` | WAL-only reopen, flush, multiple tables, updates/deletes, compaction, duplicate transaction rejection, namespaces, invalid input, auto-flush, bounded scan, lock, empty compaction, large record, reference-map comparison |
| `tests/recovery.rs` | Every byte truncation of a small WAL, append after recovery, complete bad CRC, pre/post-manifest crash states, missing files, torn-tail reporting, failed manifest publication and poisoned handle |
| `tests/integrity.rs` | Exact changed-block diagnostic, every byte of a small SSTable authenticated, external checkpoint against replacement, exact-snapshot semantics, live block tamper, path traversal, invalid generation/order rejection |
| `tests/crash_process.rs` | Actual process termination preserves 30 acknowledged WAL-only writes; lock releases automatically |
| `tests/cli.rs` | Separate-process commands, JSON results, duplicate failure status, checkpoint, explicit tamper flag, corruption failure status, scan/delete/compact; hooks ignored by default and active only in feature builds |
| `tests/presentation.rs` | Scripted live input, real memory/disk traces, WAL reopen, inspected Merkle root, invalid numbers/negative/zero/duplicate inputs, checkpoint, copy-only tamper, isolated 18-case exercise, EOF during a form, actual automatic flush |
| `tests/range_scan.rs` | Reference-map bounds, fewer block reads, binary keys, versions/tombstones across compaction/reopen, and authentication of selected blocks |
| `tests/merkle_proof.rs` | Odd/even paths, malformed/changed proofs, wrong roots/IDs, compacted/reopened proofs, offline CLI verification |
| `tests/failure_injection.rs` | Four publication stages with exit/error actions; poisoned handles; recovered scans, tombstones, immutable receipts, and valid proofs |
| `scripts/demo.sh` | Runnable presentation sequence; requires duplicate rejection, inclusion proof verification, wrong-ID proof rejection, and exact-block tamper detection |

## Teammate review checklist

The baseline and all three teammate features are merged; ownership is recorded in [TEAM_HANDOFF.md](TEAM_HANDOFF.md). Suggested further review areas:

1. **Recovery:** repeat process termination under larger workloads; inspect crash ordering; test disk-full/write-permission failures on disposable storage. Confirm WAL is never cleared before publication.
2. **Data semantics:** test Unicode IDs, boundary sizes, repeated update/delete patterns, range boundaries, duplicate IDs after several compactions, and maximum positive amounts.
3. **Integrity and presentation:** corrupt data, metadata, roots, and manifest separately; verify external checkpoint replacement detection; reproduce the README and demo on another Linux machine.

For a bug, capture the exact commands, expected/actual result, Rust version, OS, and a minimal disposable database. Add a regression test that fails before the fix. Run formatting, Clippy, and the affected tests, then the full suite before merging.

## Interpretation of results

Automated checks establish the covered behavior, not exhaustive correctness under every failure. Small-file exhaustive byte flips do not equal a full parser fuzzing campaign. Simulated publication states are not device power-cut tests. Single-process library timings are not production throughput measurements. CI is configured in this repository; hosted results for these local changes are not claimed. See [INTEGRITY_AUDIT.md](INTEGRITY_AUDIT.md) for the current local results.
