# Running, presenting, and extending MerkleLSM

This guide covers the interactive application and all three merged teammate contributions. Bounded scans are part of the existing scan/menu flow; failure injection is an optional test feature; inclusion proofs have separate CLI commands.

## 1. Prepare the project

On Tharunn's machine:

```bash
cd /home/tharunn/Desktop/3sPADA
cargo --version
cargo build --locked
```

On a teammate's machine, clone or update the shared GitHub repository, enter the cloned project folder, and run the same Cargo commands. The locally recorded `main` already contains the baseline and three merged feature PRs. This integrity audit leaves its additional validation/documentation changes in the local working tree.

The project declares Rust 1.89 or newer and is tested on Linux. The first build downloads dependencies. Later builds can use `--offline` if the dependencies are already cached. No database server or account credentials are needed.

For a pre-presentation check:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
```

The integrated audit suite has 40 default-build test functions and 42 with all features enabled. Child-worker helpers are included in those totals; their parents exercise real subprocess failures. See [INTEGRITY_AUDIT.md](INTEGRITY_AUDIT.md) for validation details.

## 2. Start the live menu

```bash
cargo run --locked -- --db presentation-db present
```

Keep this terminal open. Enter a menu number, press Enter, and answer its prompts. Start with a new database path for a clean demonstration. Reusing a path also reuses its receipts and transaction IDs.

For another rehearsal, use a new path:

```bash
cargo run --locked -- --db target/rehearsal-02 present
```

The `target/` example keeps rehearsal files in an ignored build-output directory. The program also prints where it retains the separate presentation artifacts.

## 3. Demonstration script: what to type, observe, and explain

Use this sequence on a new database. The stated sequence numbers assume you have not made other writes.

### Step A: enter a valid transaction

Choose **1**. Enter these answers at the prompts:

| Prompt | Input |
| --- | --- |
| Transaction ID | `tx-001` |
| Sender | `Alice` |
| Receiver | `Bob` |
| Amount in integer paise | `1050` |
| Unix timestamp in milliseconds | Press Enter |
| Note | `Lunch` |

Expected: the transaction is recorded at sequence 1. The trace shows validation, duplicate lookup, entry creation, WAL append and sync, and skip-list insertion.

Say: “The amount is INR 10.50. We store whole paise to avoid decimal rounding. Before acknowledging the receipt, the program syncs its WAL record. It then keeps the recent entry in a sorted skip list.”

Recording the receipt does not move real money or verify account balances.

### Step B: inspect the internal memory structures

Choose **3**. Point out:

- Actual occupied skip-list levels and their forward links.
- Internal key `t:tx-001` and its sequence.
- The serialized receipt value.
- A complete WAL frame and its byte offset/size.

Choose **2** and enter `tx-001`. Expected: the receipt and a `Memtable HIT` trace.

Say: “A recent receipt can be returned from memory. The WAL provides recovery if that memory disappears.”

You can add `tx-002` and `tx-003` with option 1 to show more sorted keys and links. Internal keys sort by bytes, so do not describe the display as timestamp order.

### Step C: show recovery before flushing

Choose **12**, then choose **2** and enter `tx-001` again.

Expected: the handle closes without flushing; reopening replays the WAL and restores the memtable entry. The receipt is still available, and the last sequence has not restarted from zero.

Say: “This is a real reopen and WAL replay. Our separate process-crash test also terminates a writer after acknowledged writes.”

Option 12 is not a power-cut simulation. If the audience asks for the process-kill test, exit with **0** and run:

```bash
cargo test --locked --test crash_process killed_process_recovers_acknowledged_writes_and_releases_lock -- --nocapture
```

Then start `present` again with the same database path to continue.

### Step D: flush and inspect the permanent table

Choose **4**, then **5**.

Expected:

- Memory entries are flushed into a new SSTable.
- The trace shows table publication, manifest publication, and finally WAL reset.
- Table inspection shows block offsets, first keys, payload sizes, stored entries, Bloom-filter size, leaf hashes, parent hashes, and root.

Say: “A flush turns recent sorted entries into an immutable file. We save and publish that file and its manifest entry before clearing the WAL.”

Small batches usually fit in one data block; that is expected. The tree still includes leaves for the header, framed data block, metadata, and hash-list/footer.

Choose **2** and enter `tx-001` again. Expected: a memory miss followed by a Bloom MAYBE, index selection, block hash verification, and a successful block lookup.

Say: “The same request now reads a verified disk block. A Bloom filter can reject absent keys, but MAYBE is not proof that a key exists.”

### Step E: enter invalid values live

Choose **1** and repeat the valid receipt with ID `tx-001`. Expected: duplicate rejection; no new sequence and no WAL append.

Choose **1** again with ID `bad-zero`, Alice/Bob, amount `0`, Enter for timestamp, and an empty note. Expected: invalid amount; sequence unchanged.

Repeat with ID `bad-negative` and amount `-100`. Expected: rejection.

Choose **2** and enter `missing-id`. Expected: NOT FOUND. Depending on its bits, the Bloom filter may reject the key or require a block lookup.

Say: “Expected invalid inputs are handled explicitly. Missing data is different from corrupted data.”

For a malformed number such as `abc`, the menu returns immediately after the amount error. You do not need to answer the timestamp/note prompts for that failed form.

### Step F: demonstrate updates, deletion, and scans

These commands use the separate mutable demonstration namespace. They do not update financial receipts.

| Menu choice | Inputs |
| --- | --- |
| **7** | Key `status`, value `pending` |
| **4** | Flush the old value |
| **7** | Key `status`, value `reviewed` |
| **8** | Key `status` |
| **7** | Key `temporary`, value `remove-me` |
| **9** | Key `temporary` |
| **3** | Inspect memory and the deletion marker |
| **10** | Start `s`, end `u` |

Expected: `status` has the newer value `reviewed`. `temporary` is hidden by a tombstone. The range scan includes visible keys from `s` inclusive to `u` exclusive.

Say: “A tombstone stops a lookup from returning an older copy in another table. A scan must resolve all versions before deciding which keys are visible.”

Choose **11**, then **17**. Expected: full compaction merges active table streams, retains the newest versions, removes obsolete values and tombstones, and publishes the replacement before deleting retired files.

### Step G: export and verify a trusted snapshot

Choose **15** and press Enter to use the suggested checkpoint path. Copy the printed path.

Choose **16** and enter that path. Expected: exact snapshot match. Do this before another write or compaction, which would legitimately change the snapshot.

Say: “The local manifest is useful for integrity checks, but it can be rewritten alongside the tables. An independently retained checkpoint anchors the expected roots for this exact snapshot.”

### Step H: change a byte and prove detection

Choose **14**, then press Enter for table 0 and block 0.

Expected: a separate table copy is created. The display shows a changed payload byte, expected leaf hash, changed leaf hash, and a verification error naming data block 0. The original database is also verified successfully.

Choose **6** to show that the original presentation database still works.

Say: “Changing a stored byte changes its hash. The copied table fails verification, while the original still matches its expected root.”

The standalone CLI `tamper` command damages its selected original database. For this walkthrough use menu option 14, which deliberately damages a copy.

### Step I: run edge cases and the full test suite

Choose **13**. Expected: 18 named exercises pass in separate databases. They cover validation, amount limits, absence versus empty values, namespace isolation, range bounds, tombstones, compaction, checkpoints, WAL-only recovery, an incomplete tail, and a complete bad checksum.

These exercises do not write receipts into your active presentation database.

Exit with **0**, then run the full automated checks when requested:

```bash
cargo test --locked --all-targets
```

Explain one result instead of only quoting the number of passing tests. For example: “A complete record with a bad CRC is rejected as corruption; only a physically unfinished final record can be removed during recovery.”

## 4. Merged feature ownership

| Presenter | Implemented contribution | Feature notes |
| --- | --- | --- |
| Tharunn | Baseline, integration, interactive presentation | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Nirlep | Bounded skip-list/table iterators and heap-merged scans | [NIRLEP_RANGE_SCANS.md](NIRLEP_RANGE_SCANS.md) |
| Reshvanth | Optional named failure hooks and subprocess recovery tests | [RESHVANTH_FAILURE_TESTING.md](RESHVANTH_FAILURE_TESTING.md) |
| Vijay | Merkle paths, transaction block proofs, offline proof CLI | [VIJAY_MERKLE_PROOFS.md](VIJAY_MERKLE_PROOFS.md) |

The feature PRs are already integrated into `main`. Do not apply the handoff patches again. Their comparison with the merged source is recorded in [INTEGRITY_AUDIT.md](INTEGRITY_AUDIT.md). Describe supplied implementation code, individual review/adaptations, and validation accurately when explaining contributions.

## 5. Nirlep: bounded range scans

`Memtable::range` seeks through skip-list levels. `SsTable::range` uses the sparse index to select candidate blocks, then authenticates every block it reads. `Db::scan_with_block_reads` heap-merges these streams with memory, keeps the greatest sequence for each key, and suppresses tombstones after merging. `Db::scan` uses the same implementation.

Public bounds are inclusive start and exclusive end in byte order. Results remain in the mutable `k:` namespace. Equal bounds return no entries and read no blocks; reversed bounds return an input error. Results are collected in memory, and opening the database still verifies all tables.

```bash
cargo test --locked --test range_scan
cargo run --locked -- --db presentation-db scan --start s --end u
```

Menu option 10 uses the same scan and its trace reports block reads. The test compares narrow/full query block counts on a multi-block dataset, checks binary keys and version/tombstone semantics across compaction/reopen, and detects a selected block changed after opening.

## 6. Reshvanth: controlled publication failures

The `fault-injection` feature enables `MERKLELSM_FAULT_STAGE` and `MERKLELSM_FAULT_ACTION`. Default builds ignore these hooks. Named stages are `flush-before-table`, `flush-after-table`, `flush-after-manifest`, and `compact-after-manifest`. Actions are `error` (returns an I/O error and poisons the handle) and `exit` (exits with status 86 without destructors).

```bash
cargo test --locked --features fault-injection --test failure_injection -- --nocapture
```

The parent test starts isolated subprocesses for all eight stage/action combinations. Reopening must preserve acknowledged values and sequences, avoid deleted-key resurrection, reject duplicate recovered receipt IDs, and allow a new checkpoint and valid transaction proof. This demonstrates controlled process failures; it does not simulate physical device power loss.

## 7. Vijay: transaction block inclusion proofs

Use a new disposable database and new output filenames for this standalone demo:

```bash
cargo run --locked -- --db proof-demo record --id tx-proof --from Alice --to Bob --amount-paise 1050 --timestamp-ms 1700000000000
cargo run --locked -- --db proof-demo flush
cargo run --locked -- --db proof-demo checkpoint proof-checkpoint.json
cargo run --locked -- --db proof-demo prove tx-proof proof.json
cargo run --locked -- verify-proof tx-proof proof.json proof-checkpoint.json
cargo test --locked --test merkle_proof
```

`prove` requires a flushed transaction; `checkpoint` also flushes. `verify-proof` does not open a database. It selects the expected table root from the independently trusted checkpoint, validates path shape, authenticates the framed block, decodes its ordered entries, and checks the requested receipt ID and fields. Change a framed-block byte in a copy of the proof JSON and repeat verification to demonstrate rejection.

Keep the checkpoint independently of the proof sender. A proof reveals the whole block, potentially including other receipts. It establishes inclusion in that snapshot, not freshness, balances, absence, or consistency across snapshots. A new proof after compaction needs the corresponding new checkpoint; a previously exported proof remains tied to its original checkpoint. Proof construction reads/verifies the table; offline verification is compact. The interactive menu does not add proof options.

## 8. Integration validation

```bash
git diff --check
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo test --locked --all-targets --all-features
cargo test --locked --doc
bash scripts/demo.sh
```

The demo now includes proof export, successful offline verification, and rejection of a different transaction ID before its deliberate corruption exercise. CI runs strict Clippy across all features and tests both default and feature-enabled builds. See [TESTING.md](TESTING.md) for coverage and [INTEGRITY_AUDIT.md](INTEGRITY_AUDIT.md) for the recorded local results.
