# Team implementation and presentation handoff

## Updated ownership

Tharunn D owns the initial working implementation. To give all four members substantial engineering ownership, the proposed next phase assigns implementation work to the other three members as well as review and debugging. These enhancements are planned, not already completed.

Follow [RUN_AND_IMPLEMENT.md](RUN_AND_IMPLEMENT.md) for a complete input-by-input demonstration, file ownership, implementation order, expected behavior, and acceptance checks. The existing interactive demo is available now; optimized range scans, controlled failure hooks, and Merkle inclusion proofs are proposed extensions.

## Proposed GitHub issues

These are ready to copy into GitHub; no issues or messages have been posted.

| Issue title | Suggested owner | Acceptance evidence |
| --- | --- | --- |
| Add controlled failure injection and recovery verification | Reshvanth Yeddla | Stop a disposable writer at named publication stages; recover acknowledged records; test uncertain I/O outcomes |
| Implement efficient bounded range scans | BODDUPALLLY NIRLEP | Correct lower-bound iterators and merged versions; reference-map agreement; fewer measured block reads on a narrow scan |
| Implement block-level Merkle inclusion proofs | Vijay Raghava Reddy | Generate and verify receipt-block proofs against an independently trusted root; reject malformed/changed proofs |
| Integrate enhancements and freeze the presentation version | Tharunn D | Shared adapters reviewed; default and feature-specific checks pass; documentation and menu match implemented features |

## Suggested implementation branches

After the baseline has been committed and published to the shared base branch, each feature owner can run the appropriate command in their own clone:

```bash
git switch -c feat/recovery-failure-testing
git switch -c feat/range-scans
git switch -c feat/merkle-proofs
```

Choose one command per feature owner; do not run all three as a sequence to assign work. For each implementation or fix:

```bash
git status
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
git add path/to/changed-file path/to/regression-test
git commit -m "Fix <specific reproduced failure>"
```

Review the diff before committing. Use the branches above for the proposed enhancements; the original plan's stub-foundation branch split describes an earlier phase. Each member should implement, test, document, demonstrate, and review real work. Publishing and merging follow the repository owner's workflow.

## Viva questions and short answers

1. **What makes this an LSM design?** Writes go to an in-memory sorted structure and WAL, then immutable sorted files; compaction merges files later.
2. **Why a skip list?** Extra forward links skip large stretches of sorted keys. Nodes live in a vector and link by index, so no unsafe pointers are needed.
3. **Why write the WAL first?** Memory disappears when the process dies; the synced diary can reconstruct pending changes.
4. **Why is a complete bad CRC an error?** A full record with changed bytes is corruption, not merely an unfinished append.
5. **Why keep a tombstone?** Without it, a search could return a deleted key's old value from an older table.
6. **When may a tombstone disappear?** During full compaction after all older table versions are included.
7. **Can a Bloom filter prove a key exists?** No. It can say definitely absent or maybe present; the reader checks the actual block.
8. **What does the Merkle root cover?** Header, framed data blocks, metadata, saved hashes, and footer. Its own field is checked separately against the expected root.
9. **What if the attacker edits the saved hashes too?** The root still differs from an independently retained checkpoint. A locally rewritten manifest alone is not a trusted anchor.
10. **Does this transfer money?** No. It records audit events with integer paise; real payment authorization and balances are outside scope.
11. **Why can read commands change a file?** Opening repairs an incomplete WAL tail, which is why even read commands acquire the exclusive lock.
12. **Why might a new valid write fail an old checkpoint check?** The checkpoint represents one exact snapshot, not every legitimate future state.

## Suggested seven-minute demonstration

Start `cargo run -- --db presentation-db present` for the interactive menu. Follow [PRESENTATION.md](PRESENTATION.md) for exact inputs. Option 13 runs isolated edge-case exercises; option 14 corrupts a copy so the presentation can continue afterward. Teammates should also rehearse this menu on their machine.

1. Explain the diary, sorted pad, folders, and seals (one minute).
2. Show `record`, `transaction`, and duplicate rejection (one minute).
3. Show persistence across CLI invocations, `flush`, and `stats` (one minute).
4. Show mutable updates, deletion, range scan, and compaction (one minute).
5. Export and verify an external checkpoint (one minute).
6. Deliberately tamper with a disposable table; show the failing block (one minute).
7. Explain trust boundaries, teammate review, and future work (one minute).
