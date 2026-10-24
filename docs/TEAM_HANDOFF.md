# Team implementation and presentation handoff

## Integrated ownership

The working baseline and all three teammate feature PRs are merged into local `main` at `45a4c62`. The local `origin/main` tracking reference also points there; this audit does not query live GitHub state.

| Owner | Contribution | Feature commit | Merge commit |
| --- | --- | --- | --- |
| Tharunn D | Baseline storage, ledger, CLI, presentation, integration | `953d390` | `ef4fb28` |
| BODDUPALLLY NIRLEP | Bounded range scans | `48dc645` | `c20ea44` |
| Reshvanth Yeddla | Failure injection and recovery tests | `bd0a053` | `45a4c62` |
| Vijay Raghava Reddy | Merkle transaction block proofs | `71419cc` | `8cd75e5` |

The recorded feature merge order was Vijay, Nirlep, then Reshvanth. The supplied handoff recommends Nirlep, Reshvanth, then Vijay for applying raw patches. Comparing a reconstructed application in that order with the merged source found all changes present; the only difference is the order of the `range` and `inclusion_proof` methods. See [INTEGRITY_AUDIT.md](INTEGRITY_AUDIT.md) for evidence and limits.

Do not reapply the patches to current `main`. Use a fresh branch from the integrated project for further changes and review the diff before committing. Accurately distinguish supplied implementation code from each member's own review, adaptations, tests, and understanding.

## Validation before another merge

```bash
git diff --check
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
cargo test --locked --doc
bash scripts/demo.sh
```

Follow [RUN_AND_IMPLEMENT.md](RUN_AND_IMPLEMENT.md) for feature contracts and demos. Menu option 10 uses bounded scans. Failure injection is exercised through feature-enabled tests. Inclusion proof export/verification use the CLI and do not add menu options.

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
