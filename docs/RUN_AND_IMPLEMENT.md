# Running, presenting, and extending MerkleLSM

This guide covers two tasks: demonstrating the application that already works, and implementing the proposed teammate contributions. The existing interactive application is ready to run. The three enhancement assignments below are proposals and have not yet been implemented.

## 1. Prepare the project

On Tharunn's machine:

```bash
cd /home/tharunn/Desktop/3sPADA
cargo --version
cargo build --locked
```

On a teammate's machine, first clone the actual GitHub repository, enter the cloned project folder, and run the same Cargo commands. The owner must publish the baseline implementation before other clones can receive it. At the time this guide was written, the implementation was still local working-tree changes; this task does not commit or publish them.

The project declares Rust 1.89 or newer and is tested on Linux. The first build downloads dependencies. Later builds can use `--offline` if the dependencies are already cached. No database server or account credentials are needed.

For a pre-presentation check:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

The last verified baseline has 33 passing test functions. Tests added by teammates will increase that number.

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

## 4. Who presents what

| Presenter | Existing demonstration | New work they can own |
| --- | --- | --- |
| Tharunn | Input, overall write path, integration | Integrate the enhancements and keep docs/demo consistent |
| Nirlep | Skip list, scans, versions, tombstones | Efficient bounded range scans |
| Reshvanth | WAL, recovery, publication ordering | Controlled failure injection and recovery verification |
| Vijay | Hashes, roots, checkpoints, copy tamper | Block-level Merkle inclusion proofs |

The existing version was implemented as the baseline. Teammates should identify their actual additions and fixes when describing contributions. The following sections are implementation assignments, not features already available in the menu.

## 5. Prepare the shared implementation workflow

Before branching, the owner should review and commit the baseline on the team's chosen integration branch, then publish it through the team's normal GitHub workflow. Others can then clone or update their copies. Sharing only the earlier README commit would not give them the working Rust project.

Each teammate starts from that same baseline and creates one branch in their own clone:

```bash
# Nirlep runs this command in his clone:
git switch -c feat/range-scans

# Reshvanth runs this command in his clone:
git switch -c feat/recovery-failure-testing

# Vijay runs this command in his clone:
git switch -c feat/merkle-proofs
```

These are three alternatives for three people, not three commands to run consecutively in one working copy.

Each feature owner should submit a small design note, implementation, meaningful tests, updated documentation, a demo of their change, and a review of someone else's change. Commit the actual work in sensible steps. Each pull request should explain the problem, behavior, validation, and limitations.

`db.rs`, `sstable/mod.rs`, and presentation integration are shared touchpoints. Agree on the new method names and types first. Nirlep and Vijay should coordinate their SSTable changes; Tharunn can integrate their small adapters and resolve overlaps. No change should silently redefine the existing on-disk layout or Merkle rules.

## 6. Nirlep: implement efficient range scans

### Current behavior

`Db::scan` uses `merged()` to read entries into a `BTreeMap` before applying range bounds. It is correct for the covered cases, but even a narrow range can read unrelated entries.

### Files to study and extend

- `src/memtable/mod.rs`: skip-list search, forward links, and iteration.
- `src/sstable/mod.rs`: sparse index, block reads, ordered iterator.
- `src/db.rs`: `scan`, `merged`, namespace rules, and heap merge used by compaction.
- New `tests/range_scan.rs` and existing persistence tests.

### Implementation order

1. Define lower-bound range iteration for the memtable. Reuse skip-list search to find the first key at or above the start, then follow level-zero links until the end.
2. Add an SSTable range iterator. Binary-search the sparse index and include the block that can contain the start, even when its first key is smaller. Read successive candidate blocks only until the upper bound is reached.
3. Merge the bounded streams using a heap. Group equal keys, retain the greatest sequence, and only then suppress tombstones.
4. Apply the internal `k:` namespace correctly. Public scan bounds describe user bytes; an unbounded end must still stop at the end of that namespace.
5. Keep `Db::scan`'s public return type and inclusive-start/exclusive-end behavior. It can still collect matching results into a vector. Do not claim bounded total memory while returning all results in that vector.
6. Add observable block-read counts for verification. Use them to show that a narrow query reads fewer blocks than a full scan on the same multi-block dataset.

A Bloom filter is a point-membership shortcut; it must not be used to reject a whole key interval based on one queried key.

### Completion checks

Test missing bounds, equal bounds, reversed bounds, a start inside a block, an end on a block boundary, empty results, byte/Unicode ordering, repeated updates, memory/disk overlap, and a tombstone hiding an older value. Compare results against a reference map.

Run after those tests have been created:

```bash
cargo test --locked --test range_scan
```

For the new demo, create a multi-block dataset and enter a narrow range through menu 10. Show the correct values and measured reduction in block reads. The block-read counter/demo still needs implementing.

## 7. Reshvanth: implement controlled failures and recovery evidence

### Current behavior

Existing tests already cover truncated WAL records, complete CRC failures, reconstructed publication states, a forced manifest-write failure, and termination of a real process. Extend this with precisely controlled failures during the real flush and compaction paths.

### Files to study and extend

- `src/wal/mod.rs`: append, replay, sync, reset.
- `src/manifest.rs`: atomic write and directory sync.
- `src/db.rs`: `flush_inner`, `compact_inner`, poisoned handle, replay sequence handling.
- `tests/recovery.rs`, `tests/crash_process.rs`, and new `tests/failure_injection.rs`.

### Implementation order

1. Document the persistence stages and the invariant expected at each one.
2. Design named failure hooks. Keep them disabled in normal builds; an explicit Cargo feature such as `fault-injection` can enable the test machinery. That feature does not exist yet.
3. Put hooks at meaningful boundaries: before table publication; after table publication but before manifest publication; after manifest publication but before WAL reset; and after compaction publication but before retired-file deletion.
4. In subprocess tests, stop the writer at the chosen hook without running cleanup. Record which operations were already acknowledged before termination.
5. Reopen the database and verify all acknowledged writes, sequence continuity, correct table visibility, and harmless orphan files.
6. Separately simulate returned I/O errors. Verify the handle becomes poisoned and does not continue writing until reopened.
7. If a case fails, fix the root cause and keep the regression test. Do not make recovery pass by silently ignoring a complete bad record.

The hooks must preserve the normal publication order. A default production run must not terminate because someone accidentally set a testing environment variable. Feature-gated integration tests need a library feature; `#[cfg(test)]` alone does not expose library hooks to an integration-test binary.

### Completion checks

| Failure boundary | Expected recovery |
| --- | --- |
| Before manifest publication | Old manifest remains authoritative; WAL retains acknowledged writes |
| After manifest publication, before WAL reset | Published tables are visible; older WAL records replay idempotently |
| After compaction publication, before cleanup | Replacement remains authoritative; leftover old files are ignored |
| Returned write error | Current handle refuses continued operations until reopened |

After adding the feature and test file, the proposed command is:

```bash
cargo test --locked --features fault-injection --test failure_injection -- --nocapture
```

This command is not available until the feature is implemented. The teammate's live demonstration should name the injected failure stage, stop a disposable writer, reopen, and show the acknowledged records that survived. Describe it as controlled process failure, not proof of hardware power-loss behavior.

## 8. Vijay: implement Merkle inclusion proofs

### Current behavior

The code verifies full SSTables and displays all Merkle levels. It does not yet export or verify an inclusion proof for one transaction's data block.

### Files to study and extend

- `src/merkle/mod.rs`: leaf/parent hashing, odd-node duplication, root construction.
- `src/sstable/mod.rs`: block lookup, framed block bytes, saved leaves, and inspection tree.
- A new proof module or proof types in the Merkle module.
- Small agreed adapters in `src/db.rs`, `src/main.rs`, and `src/presentation/`.
- New `tests/merkle_proof.rs`.

### Implementation order

1. Agree on proof fields: format version, leaf index, leaf count, sibling hashes and their ordering, table identity, and the framed block bytes or a clearly defined external block input.
2. Generate the path from the data-block leaf to the root. A block at index B uses leaf index B+1 because the header is the first leaf.
3. Include the actual complete leaf set: header, data blocks, metadata, and the final hash-list/footer leaf. Reuse the existing domain prefixes and odd-node duplication rule.
4. Verify the leaf bytes, sibling path, index/count bounds, expected number of levels, and independently supplied trusted root.
5. Decode the authenticated block and check that it contains the requested transaction key/value. Block membership alone must not be presented as proof of a different receipt.
6. Define behavior for pending records: either require the user to flush first or explicitly flush before producing a proof.
7. Add proof JSON export/import and CLI/presentation integration after the library implementation works.

Do not accept a root simply because it is stored inside the same proof document. Select the expected root from an independently trusted checkpoint and verify table identity. The proof establishes inclusion of a block in that snapshot; it does not establish absence of other records or consistency between successive snapshots.

### Completion checks

Test first/middle/last data blocks, odd and even leaf counts, altered block bytes, a wrong sibling, a wrong root, malformed indices/counts/path lengths, unknown transaction IDs, and unflushed entries. Verification against a wrong checkpoint must fail.

Run after creating the test:

```bash
cargo test --locked --test merkle_proof
```

For the new live demo, export a proof for a chosen receipt, verify it against the trusted checkpoint, change its authenticated input, and show verification failing. Agree on the exact CLI names during implementation; no `prove` or `verify-proof` command is currently implemented.

## 9. Tharunn: integrate and keep the demo accurate

1. Review the three interface notes before shared files change.
2. Review and integrate one feature at a time, resolving changes in shared database/table/presentation modules.
3. Preserve the existing default commands and data format unless an explicit migration decision is made.
4. Run formatting, Clippy, default tests, and any feature-enabled recovery tests after integration.
5. Update the live menu, README, ReportOne, and this guide to match actual available commands and outputs.
6. Rehearse the entire presentation with each member explaining their implemented change.

For each ordinary feature change:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
bash scripts/demo.sh
```

If failure injection is added behind a feature, also add its explicit feature-enabled validation to CI. Passing default tests alone would not exercise disabled hooks.

## 10. What each teammate hands back

- A working feature branch containing their actual implementation commits.
- A short design note stating the problem, chosen approach, and limitations.
- Tests demonstrating correct behavior and meaningful failure cases.
- A few commands or menu steps that reproduce their new functionality.
- A review of another teammate's work.
- An explanation they can deliver in the viva without reading code aloud.

Use commit history, reviewed changes, tests, and demonstrations as evidence of contribution. Separate the original baseline from later work accurately; equal participation should come from real ownership and effort, not relabeling the initial implementation.
