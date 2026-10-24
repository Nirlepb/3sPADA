# MerkleLSM — Tamper-Evident Audit Ledger in Rust

A working educational LSM database with a safe skip-list memtable, durable write-ahead log, immutable sorted tables, sparse indexes, Bloom filters, and Merkle verification. The financial ledger stores append-only transaction events in integer INR paise and rejects duplicate IDs. Separate key/value commands demonstrate updates, tombstones, scans, and heap-based compaction.

Start with [ReportOne.md](ReportOne.md) for the beginner explanation of every directory and file. The current specification is [docs/ROADMAP.md](docs/ROADMAP.md); the supplied plan is preserved in [docs/ORIGINAL_PLAN.txt](docs/ORIGINAL_PLAN.txt).

## Team and merged contributions

| Name | Roll number | Merged contribution |
| --- | --- | --- |
| BODDUPALLLY NIRLEP | CB.SC.U4CSE25313 | Efficient bounded range scans, tests, and storage demonstration |
| Reshvanth Yeddla | CB.SC.U4CSE25347 | Controlled failure injection, recovery verification, and regression fixes |
| Tharunn D | CB.SC.U4CSE25359 | Baseline implementation, shared integration, documentation, and full demonstration |
| Vijay Raghava Reddy | CB.SC.U4CSE25363 | Merkle inclusion proofs, tests, and verification demonstration |

The baseline and all three teammate enhancements are merged. Bounded scans use the existing scan command and menu option 10; failure injection is enabled only with the `fault-injection` feature; Merkle proofs use the `prove` and `verify-proof` CLI commands. See [RUN_AND_IMPLEMENT.md](docs/RUN_AND_IMPLEMENT.md) for the integrated run guide and [docs/INTEGRITY_AUDIT.md](docs/INTEGRITY_AUDIT.md) for the patch comparison and validation results.

## Build and test

Use Linux and Rust 1.89 or newer with Cargo. The implementation uses OS file locks and directory syncing; Linux is the tested platform. Java, CMake, and system OpenSSL are not application dependencies. A normal Rust toolchain still needs its platform linker.

```bash
cd /home/tharunn/Desktop/3sPADA
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
cargo test --locked --doc
bash scripts/demo.sh
```

The first build downloads dependencies. Later builds can use `--offline` if those dependencies are cached. `Cargo.lock` records the resolved versions.

## Interactive live presentation

```bash
cargo run --locked -- --db presentation-db present
```

The menu prompts for transaction ID, sender, receiver, integer paise, timestamp (Enter uses the current time), and a note. It displays events from the actual storage operations and supports inspection of skip-list links, WAL frames, table blocks, sparse indexes, Bloom-filter decisions, and full Merkle hashes.

Choose **13** for 18 real edge-case exercises in a separate database. Choose **14** to corrupt a table copy and demonstrate detection while retaining a usable main database. Choose **12** to close/reopen without flushing and show WAL recovery. Input errors return to the menu; **0** exits.

Follow [docs/PRESENTATION.md](docs/PRESENTATION.md) for exact sample inputs and a presentation sequence. Artifact copies, exercise databases, and default checkpoints are retained in `presentation-artifacts/` beside the database. The `interactive` and `presentation` subcommand names are aliases for `present`.

## Record and retrieve a transaction

```bash
cargo run -- --db demo-db init
cargo run -- --db demo-db record \
  --id tx-001 --from Alice --to Bob \
  --amount-paise 1050 --timestamp-ms 1700000000000 --note Lunch
cargo run -- --db demo-db transaction tx-001
cargo run -- --db demo-db transactions
cargo run -- --db demo-db flush
cargo run -- --db demo-db verify
cargo run -- --db demo-db stats
```

`1050` paise means ₹10.50. Timestamps are supplied as Unix milliseconds. Recording an event does not transfer money or calculate account balances. Repeating `tx-001` fails, including after restart and compaction. Every successful write has already called `sync_all()` on the WAL; closing the CLI does not need to flush the memtable.

## Demonstrate the LSM data structure

```bash
cargo run -- --db demo-db put note pending
cargo run -- --db demo-db flush
cargo run -- --db demo-db put note reviewed
cargo run -- --db demo-db put temporary remove-me
cargo run -- --db demo-db delete temporary
cargo run -- --db demo-db get note
cargo run -- --db demo-db scan --start a --end z
cargo run -- --db demo-db compact
```

The mutable key/value area is isolated from financial transactions. `get` prints a JSON string or `null`; scans produce one JSON object per line. Transaction commands print JSON. A scan's start is inclusive and its end is exclusive. Keys sort by bytes. CLI strings use UTF-8; arbitrary binary values are supported by the library but displayed with lossy UTF-8 conversion by the CLI.

## Checkpoints and the tamper demo

```bash
# Use a new filename outside the database directory.
cargo run -- --db demo-db checkpoint checkpoint-001.json
cargo run -- --db demo-db verify --checkpoint checkpoint-001.json

# Intentionally damage a disposable database.
cargo run -- --db demo-db tamper --table 0 --block 0 --allow-demo-tamper
cargo run -- --db demo-db verify
```

The final command exits with an error naming the table and data block. Table numbers follow manifest order (oldest first); block numbers start at zero. `bash scripts/demo.sh` performs the demonstration in a temporary database and cleans up its own files.

Local verification detects corruption relative to the local manifest. An attacker able to replace the tables **and** manifest can make a new self-consistent database. Keep the exported checkpoint in a separate trusted location to detect that replacement or rollback. A checkpoint describes one exact flushed snapshot; later legitimate writes or compaction also make it differ. Checkpoints are not signed, and CRC32 is not a cryptographic safeguard for unflushed WAL records. See [SECURITY.md](docs/SECURITY.md).

## Library and timing examples

```bash
cargo run --example basic -- /tmp/my-example-ledger
cargo run --release --example benchmark -- /tmp/my-new-benchmark-db 1000
cargo run -- --help
```

The benchmark requires a new directory and retains it for inspection. It times durable writes, reads, flush, and compaction separately. Results depend on the filesystem and hardware; it is a demonstration, not a rigorous performance study.

## Documentation

| Document | Purpose |
| --- | --- |
| [ReportOne.md](ReportOne.md) | Beginner explanation, complete file map, implementation plan, walkthrough, Rust cheatsheet |
| [ROADMAP.md](docs/ROADMAP.md) | Requirements and resolved differences in the original plan |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Module connections and durability order |
| [FILE_FORMAT.md](docs/FILE_FORMAT.md) | Exact bytes, Merkle rules, manifest/checkpoint schema |
| [SECURITY.md](docs/SECURITY.md) | Trust boundaries and limitations |
| [TESTING.md](docs/TESTING.md) | Automated coverage and final teammate review checklist |
| [TEAM_HANDOFF.md](docs/TEAM_HANDOFF.md) | Merged feature ownership, validation, and viva questions |
| [PRESENTATION.md](docs/PRESENTATION.md) | Interactive menu, sample inputs, and live demonstration sequence |
| [RUN_AND_IMPLEMENT.md](docs/RUN_AND_IMPLEMENT.md) | Full run/demo guide and merged teammate feature contracts |

There is no networking, asynchronous runtime, background worker, automatic leveled compaction, or encryption. Inclusion proofs are available through the CLI. Tables are capped at 256 MiB; scans and compaction collect output in memory. Every database handle takes an exclusive lock. These choices keep the implementation appropriate for a data-structures project.
