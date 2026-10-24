# Interactive presentation guide

For a combined run-through and the merged teammate features, see [RUN_AND_IMPLEMENT.md](RUN_AND_IMPLEMENT.md). Menu option 10 uses bounded scans and reports block reads in its trace. Proof export/verification use the separate CLI; failure injection is an optional test feature. The interactive menu retains its existing options.

## Start here

```bash
cd /home/tharunn/Desktop/3sPADA
cargo run --locked -- --db presentation-db present
```

The program stays open and asks for your inputs. Type a menu number and press Enter. The same database path retains your data between sessions. To rehearse from an empty database, choose another new path, such as `--db presentation-rehearsal-2`, rather than deleting your existing data.

The menu has real operations, inspection views, and separate edge-case exercises. The “UNDER THE HOOD” messages are recorded by the real database functions and displayed after the chosen operation completes. Success messages appear only after their corresponding operation succeeds. They are not prerecorded animation or simulated storage.

## Menu reference

| Option | Action | Main thing to explain |
| --- | --- | --- |
| 1 | Enter a financial transaction | Validation, unique ID, sequence, synced WAL, skip-list insertion |
| 2 | Enter a transaction ID to find | Actual memory hit or disk lookup path |
| 3 | Inspect memory and WAL | Actual skip-list levels, sorted entries, WAL offsets and frame sizes |
| 4 | Flush | Table first, then manifest, then WAL reset |
| 5 | Inspect tables | Block boundaries, sparse index, Bloom size, leaves, parent levels, root |
| 6 | Verify current state | Tables against local manifest, plus WAL CRCs |
| 7 | Put/update a demonstration key | Increasing sequences and replacement in the mutable namespace |
| 8 | Find a demonstration key | Bloom decisions, binary search, hash check, tombstone result |
| 9 | Delete a demonstration key | Write a tombstone rather than returning an old value |
| 10 | Range scan | Start inclusive, end exclusive; newest sequence wins |
| 11 | Compact | Full heap merge, removal of obsolete versions/tombstones |
| 12 | Close and reopen without flushing | WAL replay recreates pending memory entries |
| 13 | Run 18 real edge-case exercises | Named PASS/FAIL results in separate databases |
| 14 | Corrupt a table copy | Expected and changed hashes; exact damaged block; original still verifies |
| 15 | Create an external checkpoint | Flush and export exact trusted snapshot metadata |
| 16 | Verify against a checkpoint | Compare current state with independently retained snapshot |
| 17 | Statistics and transaction list | Current sequence versus persisted sequence, generation, memory, tables |
| 0 | Exit | Acknowledged writes remain durable even without flushing |

## Suggested live demonstration

### 1. Enter a receipt

Choose **1** and enter:

```text
Transaction ID: tx-001
Sender: Alice
Receiver: Bob
Amount in integer paise: 1050
Unix timestamp: [press Enter]
Note: Lunch reimbursement
```

Explain that 1,050 paise means INR 10.50. The default timestamp is the system's current Unix time in milliseconds. This stores an audit event, not a real money transfer.

Point to the trace: financial validation, duplicate lookup, sequence assignment, WAL framing/sync, and skip-list insertion. Repeat with IDs `tx-002` and `tx-003`, or let the audience choose the values. IDs must be unique across this database, including previous sessions.

### 2. Show memory and recovery before flushing

Choose **3**. The display shows real forward links at each occupied skip-list level and sorted bottom-level entries. Higher levels skip some keys. The WAL shows complete frames, byte offsets, sequences, and verified CRCs.

Choose **2**, enter `tx-001`, and point to the memory-hit trace.

Choose **12**. The handle closes without flushing, then reopens. The memtable is reconstructed from WAL records. Choose **2** again and retrieve the same receipt. Explain that this is a normal reopen exercise. The separate automated crash test also kills a real writer process; this menu action does not pretend to cut power.

### 3. Flush and show the disk read path

Choose **4**, then **5**. Explain the order shown in the trace: build and sync an SSTable; publish the manifest; only then truncate the WAL. The memtable becomes empty.

Option **5** shows block offsets, first keys, payload sizes, entries, Bloom-filter size, each block leaf hash, and all levels of the Merkle tree. The final node is the table root. The leaf order is header, data blocks, metadata, then the hash-list/footer leaf. Odd nodes at a level are duplicated.

Choose **2**, enter `tx-001` again, and contrast the new trace with the earlier memory hit. This time the memtable misses, the Bloom filter says MAYBE, the sparse index chooses a block, its hash is verified, and binary search finds the entry.

Choose **2** with an unknown ID to show absence. A Bloom filter may reject that key immediately or produce a false positive that still requires checking the block. Do not promise that every unknown key will be rejected by the filter.

### 4. Enter bad inputs yourself

Choose **1** and repeat `tx-001` with otherwise valid fields. It must reject the duplicate before another WAL append. The sequence before and after is displayed.

Choose **1** again with a new ID and amount `0`, then another new ID with amount `-100`. Complete the remaining prompts. Both must be rejected. An amount such as `10.50` or `abc` produces a number-input error and immediately returns to the menu; use integer paise for valid input.

You can also try equal sender/receiver names or a blank ID. Invalid operations should not advance the sequence. An unknown menu number returns a helpful message instead of closing the program.

### 5. Show versions, tombstones, range scan, and compaction

Choose **7**: key `status`, value `pending`. Choose **4** to put that version on disk.

Choose **7**: key `status`, value `reviewed`. Choose **8** and enter `status`; the newer value wins.

Choose **7**: key `temporary`, value `remove-me`. Choose **9** and enter `temporary`. Choose **3** to inspect the tombstone.

Choose **10**. Enter `s` as the start and `u` as the end. The scan should include `status` and exclude deleted `temporary`. Alternatively, press Enter at both prompts for an unbounded scan.

Choose **11**. Explain the heap merge, greatest-sequence rule, durable publication, and safe removal of tombstones during a merge of every active table. Inspect statistics with **17**.

### 6. Checkpoint and safe tamper exercise

Choose **15** and press Enter for the suggested checkpoint filename. Copy that printed path. Choose **16** and enter the same path. It should match before any subsequent change to the snapshot.

Choose **14**, press Enter for table 0, and press Enter for block 0. The program flushes first and copies that table into the presentation artifact directory. It flips one payload byte in the copy, prints old/new byte values and expected/changed leaf hashes, and requires verification to identify the damaged block.

The original database is then verified too. Choose **6** or retrieve a receipt to show it remains usable. This menu exercise differs from the standalone `tamper` command, which intentionally damages the selected original database.

Say explicitly that a local manifest is not an independent trust anchor. An attacker who rewrites data and manifest together can make internally consistent replacements. An independently retained checkpoint can detect changes to that exact snapshot. Later legitimate writes or compaction change the snapshot too.

### 7. Run the edge-case exercises

Choose **13**. It uses separate, newly created databases, runs real operations, and prints 18 named checks:

1. Duplicate ID rejected without a new sequence.
2. Zero amount rejected.
3. Negative amount rejected.
4. Equal source and destination rejected.
5. Blank ID rejected.
6. Missing ID returns absence.
7. Maximum positive `i64` paise round-trips exactly.
8. Empty value remains distinguishable from absence.
9. Mutable operations cannot overwrite/delete a financial receipt.
10. Empty mutable key rejected.
11. Reversed scan bounds rejected.
12. Tombstone hides an old disk value.
13. Compaction does not resurrect a deleted key.
14. A flushed checkpoint matches.
15. A later legitimate write differs from the old exact snapshot.
16. WAL-only writes recover without SSTables.
17. A two-byte incomplete WAL tail is removed while complete data survives.
18. A complete bad CRC is reported as corruption rather than discarded.

These are focused presentation exercises. To show the full automated suite, exit with **0** and run:

```bash
cargo test --locked --all-targets
```

For just the actual process-kill test:

```bash
cargo test --locked --test crash_process killed_process_recovers_acknowledged_writes_and_releases_lock -- --nocapture
```

## Rehearsal tips

- Try the menu once before presenting; the first build may download/compile dependencies.
- Keep a new database path for each clean rehearsal. Existing receipts keep their IDs.
- Use several small receipts to explain sorting, and option 5 to show the resulting block/tree structure. Small batches often fit in one data block, which is expected.
- To demonstrate an automatic flush, start a separate session with `cargo run -- --db auto-flush-demo --memtable-bytes 256 present`. A receipt can reach that threshold quickly; check the printed trace rather than promising a fixed record count.
- Inspection output limits long rows and large blocks to keep the terminal readable. Full hashes are shown; very long entry values are shortened for display.
- The artifact directory beside the database retains checkpoints, copied tables, and the edge-case databases for inspection. It does not erase earlier presentation runs.
- If stdin closes midway through a form, the session exits without submitting an incomplete transaction.
