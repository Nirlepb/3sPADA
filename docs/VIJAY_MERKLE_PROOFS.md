# Transaction block inclusion proofs

Proof JSON contains version, table filename, complete framed block, leaf index/count and sibling hashes. The path includes the header, all data blocks, metadata and hash-list/footer leaf under the existing hash domains and odd-node duplication rule. Verification obtains the root from an independently retained checkpoint and decodes the authenticated block to check the requested transaction. Pending receipts require an explicit flush. The disk format is unchanged.

Demo with fresh paths, from the project root:

```bash
cargo run -- --db target/proof-demo record --id tx-001 --from Alice --to Bob --amount-paise 1050 --timestamp-ms 1700000000000
cargo run -- --db target/proof-demo checkpoint target/proof-checkpoint.json
cargo run -- --db target/proof-demo prove tx-001 target/tx-001-proof.json
cargo run -- verify-proof tx-001 target/tx-001-proof.json target/proof-checkpoint.json
cargo test --locked --test merkle_proof
```

The verifier does not open a database. Edit a framed_block byte in a copy of the JSON and repeat verification to demonstrate rejection. Use the CLI for this new demo; the interactive menu retains its existing options. Checkpoint and proof outputs require new filenames. Proofs reveal the whole block (possibly including other receipts). They establish inclusion in that snapshot, not freshness, absence, balances, or consistency across snapshots. Proof construction still reads/verifies the table; only verification is compact.
