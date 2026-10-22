# Report One: Building and Understanding MerkleLSM

## 1. What this project does

Imagine your class has a notebook that records who paid whom. Alice pays Bob ₹10.50, and you write down a receipt. Later, someone asks two questions:

1. “Can you find Alice's receipt?”
2. “Can you tell whether somebody secretly changed it?”

This project answers those questions with a small database written in **Rust**. A database is an organized place to save and find information. Our database is called **MerkleLSM**.

It saves financial **audit events**. An audit event is a written record of something that happened. Recording a payment here does not send real money. The program does not log into a bank or work out account balances.

The main data-structure design is an **LSM tree**. Its supporting structures are a **skip list**, **Bloom filter**, **sparse index**, **Merkle tree**, and **heap**. Each has a specific job, explained below.

The project is a complete working library and terminal application. It includes tests, examples, a presentation script, and documentation. There are no unfinished placeholder modules.

## 2. What I found in the supplied plan

The requested path was `/home/tharunn/Desktop/DSA_plan/Plan.txt`. The actual available file was `/home/tharunn/Desktop/DSA_plan/Plan`. It was ordinary UTF-8 text, not a PDF. I read that file and kept an exact copy in [docs/ORIGINAL_PLAN.txt](docs/ORIGINAL_PLAN.txt).

The repository `/home/tharunn/Desktop/3sPADA` initially contained a README and its Git history. There was no `Cargo.toml`, source code, test suite, or existing Rust module to reuse. The original README described the idea but still listed C++/Java build tools. The new README describes the Rust implementation.

The source plan combined several older instructions: inspect and stop, build only a skeleton, let four people implement separate modules, and later integrate everything. Your latest instruction changes that process. This implementation follows the new arrangement:

| Person | Responsibility in the updated plan |
| --- | --- |
| Tharunn D | Own the complete implementation and integration |
| BODDUPALLLY NIRLEP | Final review, edge-case testing, and debugging |
| Reshvanth Yeddla | Final review, edge-case testing, and debugging |
| Vijay Raghava Reddy | Final review, edge-case testing, and debugging |

The code already has automated checks. Your teammates' final debugging is an additional review stage. It does not mean handing them an untested skeleton.

After the baseline was completed, the contribution discussion proposed a larger implementation role for each teammate: Nirlep can implement efficient bounded range scans, Reshvanth can add controlled failure/recovery verification, and Vijay can implement Merkle inclusion proofs. Tharunn retains baseline and integration responsibility. These are future assignments, not completed features. [RUN_AND_IMPLEMENT.md](docs/RUN_AND_IMPLEMENT.md) gives the implementation order, files, acceptance checks, and exact live-demo inputs.

The current specification is [docs/ROADMAP.md](docs/ROADMAP.md). It also records important decisions where the original text disagreed with itself. For example, a complete WAL record with a bad checksum is an error; only an unfinished final record can be discarded during recovery.

## 3. The accountant's office analogy

Picture a busy accountant sitting at a desk.

| Office object | Program component | Its job |
| --- | --- | --- |
| Front desk | `Db` | Receives requests and coordinates the work |
| Receipt form | `Transaction` | Holds the payment's ID, accounts, amount, and time |
| Numbered diary | WAL | Saves every new change before saying “done” |
| Sorted pad on the desk | Memtable | Makes recent changes quick to find |
| Express links between pad entries | Skip list | Skips over many entries during a search |
| Permanent sorted folder | SSTable | Stores a batch of entries on disk |
| Folder contents page | Sparse index | Tells us which block might contain a key |
| Quick “possibly here” label | Bloom filter | Avoids opening folders that definitely lack the key |
| Seal for every page and whole folder | Merkle hashes | Reveals changed bytes |
| List of active folders | Manifest | Says which table files belong to the database |
| Combining old folders | Compaction | Keeps the latest versions and removes obsolete ones |
| Inspector's separately stored seal list | Trusted checkpoint | Lets us compare against an independently retained snapshot |

This analogy is useful, but there is one difference: a computer hash is a calculated fingerprint, not a physical seal. Someone who controls all data and all local fingerprints can replace both. That is why separately retaining a checkpoint matters.

## 4. The implementation plan

The work was organized in this order so each larger component could use the smaller components below it.

| Stage | Work | Why it comes here |
| --- | --- | --- |
| 1 | Inspect the plan and repository | Understand the intended project and avoid rebuilding existing code |
| 2 | Define shared types, errors, prefixes, and binary encoding | Every module must speak the same language |
| 3 | Build skip list, WAL, Bloom filter, and Merkle functions | These are the smaller building blocks |
| 4 | Build SSTables and manifest publication | Give the database permanent, verifiable storage |
| 5 | Connect everything in `Db` | Make write, read, restart, flush, and compaction work together |
| 6 | Add financial record rules and commands | Make the storage useful as an audit ledger |
| 7 | Add checkpoints and deliberate tampering | Demonstrate both verification and its trust boundary |
| 8 | Test failure cases and write documentation | Make the work reviewable and understandable |

The finished project is **one Cargo crate**, not a workspace of separate packages. Think of a crate as one neatly packed Rust project. This crate produces a reusable library and a program you can run from the terminal.

## 5. Complete file and directory map

Every project source file is inside the repository. The tree below lists the maintained files; build output is generated separately under `target/`.

```text
3sPADA/
├── Cargo.toml                  Project settings and direct dependencies
├── Cargo.lock                  Exact resolved dependency versions
├── .gitignore                  Files Git should leave out
├── README.md                   Short introduction and runnable commands
├── ReportOne.md                This detailed beginner report
├── .github/
│   └── workflows/
│       └── ci.yml              Automated checks for GitHub
├── src/
│   ├── lib.rs                  Library's public entrance
│   ├── main.rs                 Terminal program and commands
│   ├── types.rs                Entry, Kind, Transaction, and limits
│   ├── error.rs                Shared error vocabulary
│   ├── codec.rs                Convert entries to/from bytes
│   ├── db.rs                   Connect storage pieces into one database
│   ├── manifest.rs             Active-table list and durable file publication
│   ├── bloom.rs                Bloom filter
│   ├── presentation/
│   │   ├── mod.rs              Interactive prompts, session, and copy-tamper exercise
│   │   ├── views.rs            Actual skip-list, WAL, table, and Merkle displays
│   │   └── cases.rs            18 isolated real edge-case exercises
│   ├── memtable/
│   │   └── mod.rs              Safe skip-list implementation
│   ├── wal/
│   │   └── mod.rs              Durable diary and crash replay
│   ├── sstable/
│   │   └── mod.rs              Sorted table builder, reader, and verification
│   └── merkle/
│       └── mod.rs              SHA-256 leaves, parents, and tree root
├── tests/
│   ├── common/
│   │   └── mod.rs              Temporary directories and sample transactions
│   ├── persistence.rs          Reopen, flush, scans, duplicate IDs, compaction
│   ├── recovery.rs             Torn WAL writes and publication failures
│   ├── integrity.rs            Corruption and trusted checkpoint tests
│   ├── crash_process.rs        Kill a real writer process and recover
│   ├── cli.rs                  Test the actual terminal executable
│   └── presentation.rs         Test typed inputs, traces, recovery, and menu exercises
├── examples/
│   ├── basic.rs                Small Rust program using the library
│   └── benchmark.rs            Simple write/read/flush/compaction timings
├── scripts/
│   └── demo.sh                 Complete disposable presentation demo
└── docs/
    ├── ORIGINAL_PLAN.txt       Exact copy of the source plan
    ├── ROADMAP.md              Updated specification and decisions
    ├── ARCHITECTURE.md         Module connections and operation order
    ├── FILE_FORMAT.md          Exact disk bytes and JSON shapes
    ├── SECURITY.md             What is protected and what must be trusted
    ├── TESTING.md              Test coverage and teammate checklist
    ├── TEAM_HANDOFF.md         Review tasks, branch commands, viva questions
    ├── PRESENTATION.md         Menu choices, input values, and live demo sequence
    ├── RUN_AND_IMPLEMENT.md    Full demo script and proposed teammate implementation guide
    └── VALIDATION.md           Recorded local verification results
```

The existing `.git/` directory is Git's history cabinet. It is not application code. Cargo creates `target/` for compiled programs, cached build work, and generated API documentation. It is ignored by Git. The local example runs also leave their validation databases inside `target/`.

A directory such as `src/wal/` is a module folder. Its `mod.rs` file is the folder's main Rust file. A smaller module such as `bloom` fits in `src/bloom.rs`; both forms are normal Rust module layouts. Keeping related code together makes it easier for a teammate to review one topic.

## 6. Root files and automation

### `Cargo.toml`: the project's recipe card

This tells Cargo the package name (`merklelsm`), version, Rust edition, minimum declared Rust version, dependencies, and lint rule. The edition is 2024, and the declared minimum compiler is 1.89. Local validation used Rust 1.99.1; the minimum version was not separately installed and tested.

`unsafe_code = "forbid"` tells the compiler to reject unsafe Rust in this crate. The implementation uses normal ownership and borrowing instead of manually managed pointers.

There are five direct dependencies:

| Dependency | Why it is used |
| --- | --- |
| `sha2` | Calculates SHA-256 fingerprints for Merkle trees and Bloom positions |
| `crc32fast` | Calculates WAL checksums to detect accidental record corruption |
| `clap` | Reads terminal commands and generates help/error messages |
| `serde` | Converts typed Rust structures to/from a serialized representation |
| `serde_json` | Provides the actual JSON encoding for transactions, manifests, and checkpoints |

Other names in the lockfile are supporting packages these dependencies need. This is similar to buying a bicycle whose manufacturer also uses someone else's tires.

### `Cargo.lock`: the exact shopping list

This records exact package versions and checksums. Keeping it in Git helps teammates build with the same dependency resolution. `--locked` asks Cargo to respect that recorded list.

### `.gitignore`: keep generated clutter out of history

It excludes `target/`, the default demo, benchmark, and presentation database directories, `presentation-artifacts/`, and common private-key file extensions. It does not automatically exclude every database path a user might invent. Put personal runtime databases outside tracked source or add a suitable ignore rule.

### `README.md` and `.github/workflows/ci.yml`

The README is the short starting point: build, run, test, and demonstrate. The workflow tells GitHub to check formatting, reject Clippy warnings, run tests, and run the demo on Ubuntu after a push or pull request. It is configured locally; a hosted run has not been claimed or triggered by this implementation task.

## 7. Shared vocabulary: `types.rs`, `error.rs`, and `codec.rs`

### `types.rs`: the forms everybody uses

`Entry` is the internal storage envelope. It has four fields:

| Field | Simple meaning |
| --- | --- |
| `key: Vec<u8>` | The label used to find the envelope |
| `value: Vec<u8>` | Its contents |
| `seq: u64` | The write's increasing ticket number |
| `kind: Kind` | Whether this is a value or a deletion marker |

`Vec<u8>` means a growable list of bytes. Bytes can represent text, numbers, or other data. `u64` is a whole number that cannot be negative. `Kind` has two choices: `Put` and `Delete`.

`Entry::validate` rejects empty/oversized keys, oversized values, zero sequences, and deletion markers with nonempty values. `encoded_size` counts the bytes needed by the on-disk entry form.

`Transaction` is the receipt a person understands. It contains an ID, `from`, `to`, `amount_paise`, `timestamp_ms`, and `note`. Its validation requires a usable ID, distinct nonempty account names, and a positive amount. It does not verify that those account names refer to real people.

Money uses integer paise: `1050` means ₹10.50. Floating-point numbers are avoided because they cannot represent every decimal amount exactly. The timestamp is supplied by the caller in milliseconds since the Unix epoch; it is not independently verified.

### `error.rs`: useful explanations when something fails

`Error` is one shared list of failure categories: I/O problems, corruption, tampering, invalid format, invalid input, duplicate transaction ID, unavailable lock, and poisoned handle.

An I/O error means the operating system could not complete a file operation. A poisoned handle means an earlier write failed partway through, so the program must reopen and recover before continuing. `Display` turns errors into readable terminal messages.

`Result<T>` means “either the requested answer of type T or an Error.” It lets failures travel back to the caller without pretending a write succeeded.

### `codec.rs`: packing and unpacking envelopes

Computers save bytes, not Rust structs. `encode` packs an entry into a fixed sequence of fields. `Decoder` remembers which bytes have been read and provides `take`, `u32`, `u64`, and `entry` methods.

Lengths are checked before reading slices or allocating key/value buffers. A short or malformed entry returns an error. All integers are stored little endian: the least significant byte comes first. Readers use the same rule, so the format is predictable.

## 8. `memtable/mod.rs`: the sorted pad and skip list

A memtable holds the latest in-memory entry for each internal key. It is rebuilt from the WAL after a restart.

The implementation is an actual skip list. Imagine a row of sorted houses. At ground level, paths connect each neighboring house. Higher levels have express paths that skip several houses. Searching starts on a high express path and drops down when jumping farther would pass the desired address.

The important fields are:

- `nodes`: a vector that owns all nodes.
- `head`: the starting link at each of 16 levels.
- `Node.entry`: the key/value/sequence/kind stored at that node.
- `Node.next`: optional indices of the next nodes at each level.
- `bytes`: the sum of encoded entry sizes.
- `random`: state used to choose reproducible pseudo-random node heights.

Links use vector indices, like numbered seats, instead of raw memory pointers. `None` means there is no next seat. This keeps the structure in safe Rust.

`predecessors` finds the node immediately before a target key at each level. `insert` uses those positions to connect a new node. If the key already exists, only a greater sequence replaces its entry. `get` finds one key. `iter` walks the bottom level in sorted order. `approx_size`, `len`, and `is_empty` support flush decisions and statistics.

The byte counter estimates encoded data size, not total RAM use. Rust vectors, node links, and allocator bookkeeping also use memory. The 16-level cap keeps the code small but limits how well the skip list scales to extremely large inputs.

## 9. `wal/mod.rs`: the diary that survives a crash

WAL means **write-ahead log**. “Ahead” means we write the diary before changing the in-memory pad or returning success.

A frame is one complete diary record:

```text
payload length -> encoded entry -> CRC32 checksum
```

The checksum includes both length and payload. It helps detect changed bytes but is not a secret signature.

`Wal::append` encodes an entry, builds its frame, writes it, and syncs the file. `sync` asks the operating system to persist the file. `replay` reads records in order and checks their lengths, checksums, encoding, and sequence order. It returns recovered entries, the last valid byte offset, and the size of any incomplete tail.

`Wal::open` replays first. If the last frame was unfinished, it truncates those unfinished bytes before allowing a new append. Otherwise the next good record could get stuck after an old broken record.

`reset` empties and syncs the WAL. The database calls it only after the new table and manifest are durably published.

Suppose the diary contains two complete receipts and half of a third. The two complete receipts are recovered. The unfinished third is discarded. But if the third receipt is fully present and its checksum is wrong, the program reports corruption. It does not silently discard it.

## 10. `sstable/mod.rs`: sorted permanent folders

SSTable means **sorted string table**. Here it stores byte keys and values, so values need not be strings. Once published, a table is treated as immutable by the normal database API.

`SsTable::build` accepts sorted, unique entries. It packs whole entries into blocks around 4096 bytes. A larger single entry receives its own block. It builds an index and Bloom filter, computes hashes, writes a temporary file, syncs it, renames it, and syncs the directory.

The sparse index stores each block's first key, file offset, and length. It is “sparse” because there is one index entry per block, not per stored record.

`SsTable::open` checks file size, footer offsets, header, block hashes, sorted keys, metadata, root, index consistency, count, and Bloom membership. It verifies the full file on open, then retains only the path and needed metadata/hashes.

`get` first asks the Bloom filter. If the key might be present, binary search chooses an index entry. `read_block` reads that block and checks its hash again. A second binary search finds the entry inside the decoded block.

`iter` reads blocks in order and returns their entries; scans and compaction use it. `verify` reopens and fully checks the table against expected metadata. Count methods support statistics. `block_payload_offset` supports the explicit demonstration command that changes a byte.

Files have a header, framed data blocks, metadata JSON, saved hashes, a footer, and a root. The exact byte offsets are documented in [FILE_FORMAT.md](docs/FILE_FORMAT.md). Bad lengths are bounded before use, and a table is limited to 256 MiB.

## 11. `bloom.rs`: a quick “maybe” machine

A Bloom filter is a row of bits, each either off or on. Adding a key turns on several positions calculated from its hash. Checking a key examines those positions.

If any required bit is off, the key is definitely absent. If all are on, the key might be present, so the program still checks the real data block. Different keys can share bits, which explains false positives.

`new` allocates roughly ten bits per expected key, with a small minimum. `insert` sets seven positions. `contains` checks them. `valid` rejects an empty serialized filter during table loading. Tests insert 1,000 keys and check that none disappears from the filter.

This filter is a speed shortcut. It does not replace the sorted table, index, or integrity checks.

## 12. `merkle/mod.rs`: fingerprints arranged in a tree

A hash turns bytes into a fixed-size fingerprint. Changing input bytes should change that fingerprint with overwhelming probability. It is not encryption: a hash does not hide the original stored transaction.

`hash_leaf` calculates SHA-256 with a `0x00` prefix. `hash_parent` uses `0x01` followed by two child hashes. The different prefixes keep a leaf calculation distinct from a parent calculation.

`root` repeatedly pairs hashes until one remains. When a level has an odd number, the last hash is duplicated. `verify` compares a recomputed root to an expected root. `hex` formats a hash as readable hexadecimal.

Our table has content leaves for the header, each framed data block, and metadata. Their hashes are saved. One extra leaf covers that saved hash list and the footer. The final root includes all those leaves.

Why the extra leaf? Without it, the stored hash list and footer could be left outside the protection. Why not put that extra hash into its own saved list? That would create a loop: it would depend on itself. The implementation avoids that loop.

The root itself is not included in its own calculation. Instead, it must equal both the recomputed root and the expected manifest root. A trusted external checkpoint lets us check the manifest's roots independently.

## 13. `manifest.rs`: the official folder list

`TableMeta` records a table's filename, size, and 32-byte root. `Manifest` records format version, generation, highest persisted sequence, and active tables in oldest-to-newest order.

The generation is a publication counter. It increases when flushing or compacting. The sequence is a write counter. They answer different questions: “which published arrangement?” and “which write?”

`load` and `read` decode JSON. `validate` checks version, filename shape, duplicate names, ordering, and generation/sequence consistency. Table generations must increase and cannot exceed the manifest generation. This prevents a malformed manifest from causing a new flush to reuse an active table's name.

`commit` publishes the new list using `atomic_write`. That helper writes a temporary file, syncs it, renames it, and syncs the parent directory. `sync_dir` persists directory changes such as a rename. `list_tables` returns the active list.

`lock` obtains an exclusive operating-system lock on the `LOCK` file. Another database handle must wait by retrying later or return an error. The OS releases the lock when the process dies; the file can remain on disk safely.

Files not listed in the manifest do not become active just because they exist. This is important after a crash leaves a table behind before its manifest update was finished.

## 14. `db.rs`: the front desk that joins everything

This module is the main application-facing API. It owns the directory path, options, manifest, open table readers, memtable, WAL, sequence counter, recovery statistics, and lock.

`Options` currently has a memtable byte threshold, defaulting to 1 MiB. `Stats` reports generation, current and persisted sequences, memory entries/estimated bytes, table count/bytes/blocks, and discarded recovery-tail bytes.

| Public operation | What it does |
| --- | --- |
| `open`, `open_with_options` | Lock, load tables, verify them, recover WAL, rebuild memory |
| `put`, `get`, `delete` | Work with the separate mutable key/value area |
| `record` | Validate and append a financial event with a new ID |
| `get_transaction` | Find one financial receipt by ID |
| `transactions` | List receipts in ID byte order |
| `scan` | List visible mutable keys between optional bounds |
| `flush` | Turn pending memory entries into a durable table |
| `compact` | Merge all active tables and remove obsolete versions/tombstones |
| `verify` | Recheck manifest, tables, and WAL checksums |
| `checkpoint` | Flush, verify, and export a new external snapshot file |
| `verify_checkpoint` | Compare the exact current flushed state with that file |
| `stats` | Return counters useful for inspection and the demo |

The internal `lookup` checks memory first and tables newest first. `write` gives an entry its sequence and performs WAL-before-memory ordering. `healthy` prevents continued use after an uncertain write failure. `next_manifest` advances generation safely. `merged` and `insert_newest` help scans select the newest version per key. Namespace validation prevents mixing financial records with ordinary demo values.

The prefixes are `k:` for mutable keys and `t:` for transactions. If you ask `put` to write the literal key `t:tx-001`, it actually stores `k:t:tx-001`. It cannot change the real receipt at `t:tx-001`.

### Flush order matters

Think of moving papers from the diary into a permanent folder. You must finish saving the folder and updating the official folder list before erasing the diary.

The program therefore publishes the SSTable, publishes the manifest, and only then resets the WAL. A crash before manifest publication leaves the WAL available. A crash after publication but before WAL reset leaves redundant records that replay safely skips.

### Deletion and compaction

Deleting a mutable key writes a **tombstone**, a marker saying “this key is gone.” Erasing only the newest copy would be wrong: the next lookup might find an older copy in an older table.

Compaction uses a heap to merge sorted table streams. A heap is a structure that quickly gives us the smallest next candidate. If several entries have the same key, the greatest sequence wins. Since this compaction includes every active table, old versions are gone and tombstones can be dropped safely.

The merged output is collected in memory. This keeps the code understandable but limits scalability. New files are published before old files are removed.

## 15. `lib.rs` and `main.rs`: two entrances

`lib.rs` is the entrance for another Rust program. It declares modules and re-exports important names such as `Db`, `Options`, `Stats`, `Entry`, `Transaction`, `Error`, and `Result`.

`main.rs` is the entrance for a person using the terminal. Clap reads the requested command and flags. The global `--db` chooses a database directory; `--memtable-bytes` changes the flush threshold.

The `present` command opens a menu that stays running. You enter a number, then type transaction details or a key/value when prompted. It accepts Enter for the current timestamp, shows understandable errors, and lets you continue. Run `cargo run -- --db presentation-db present` to start.

`presentation/mod.rs` manages that menu and holds the current database handle. Its input helper reads whole lines and validates numbers. Its reopen action closes the handle without flushing and opens the same files to show actual WAL recovery. Its tamper exercise copies a table before changing a byte; the original database remains available for the next question.

`presentation/views.rs` turns actual storage snapshots into readable output. It shows the skip list's real forward links, sorted memory entries, WAL frame offsets, block/index boundaries, Bloom size, and Merkle leaves and parent levels. Long values and large entry lists are shortened for terminal readability.

`presentation/cases.rs` creates separate databases and runs 18 edge-case exercises, including duplicate IDs, invalid amounts, tombstones, compaction, exact checkpoints, WAL-only recovery, a truncated tail, and a complete bad checksum. It prints PASS only after checking the real result. The exercise files remain in the presentation artifacts directory for inspection.

For the live explanations, `Db::enable_trace` enables a per-handle event list. Database and table lookup functions record what they actually did. `take_trace` drains those messages after the operation. This uses Rust's `RefCell` to allow a read operation to record diagnostic messages through an otherwise shared reference. Tracing starts disabled for normal library use. Read-only helpers provide memtable entries/levels and verified table snapshots; they do not edit the file format.

Commands include setup, financial records, mutable examples, scans, maintenance, verification, checkpoints, statistics, and deliberate tampering. Errors print to stderr and give a failure exit code. This lets scripts tell success from failure.

The `tamper` command requires `--allow-demo-tamper`. It opens a selected table and flips one byte in a selected block. It should be used only on a disposable database, because it intentionally makes that table fail verification. Ordinary commands never rewrite a published table in place.

The CLI checks that a database exists before a read command. A typo in a `get` path should not quietly create an empty database. Missing values print JSON `null`; an empty stored string prints `""`, so they are distinguishable.

## 16. Follow one receipt from beginning to end

Suppose the command is:

```bash
cargo run -- --db demo-db record \
  --id tx-001 --from Alice --to Bob \
  --amount-paise 1050 --timestamp-ms 1700000000000 --note Lunch
```

1. `main.rs` turns the words into a `Transaction`.
2. `Db` checks its fields and looks for an existing `tx-001`.
3. It creates an internal `t:tx-001` key and a new sequence.
4. `codec` packs an entry into bytes.
5. `wal` writes and syncs the framed entry.
6. `memtable` places the entry in its sorted skip list.
7. The command reports success. The next process can recover it from the WAL.
8. A later `flush` sorts the pending batch through memtable iteration and builds an SSTable.
9. The SSTable gets blocks, an index, a Bloom filter, hashes, and a root.
10. The manifest publishes the new table. Only then is the WAL cleared.
11. A later `transaction tx-001` reads the saved table and returns the receipt.
12. `verify` recomputes hashes. A changed data byte causes a failure.

Repeatedly recording `tx-001` is rejected whether the existing receipt is in memory or in a table. A teammate should be able to explain this journey without remembering every line of Rust.

## 17. Runtime files: what appears when the program runs

A database directory commonly looks like this:

```text
demo-db/
├── LOCK
├── WAL
├── MANIFEST.json
├── table-00000000000000000001.sst
└── table-00000000000000000002.sst
```

`LOCK` supports mutual exclusion. `WAL` stores pending durable records. `MANIFEST.json` lists active tables. Each `.sst` file stores one immutable sorted batch or compaction output. The number in a table name is its publication generation, not its transaction count.

Temporary names such as `.MANIFEST.json.tmp` can exist during publication. After a crash, unlisted leftovers are ignored. Automatic orphan cleanup is not implemented.

A trusted checkpoint is saved outside the database directory, for example `checkpoint-001.json` beside `demo-db/`, and should then be retained under separate control. It lists exact table names, sizes, roots, generation, and persisted sequence. It is a reference snapshot, not a second copy of transaction data and not a signed certificate.

## 18. Tests: checking the machine by trying to break it

`tests/common/mod.rs` creates isolated temporary folders and sample transactions. Each test gets its own storage so tests cannot accidentally mix records. Tests remove their temporary files after completion.

`tests/persistence.rs` checks restart, flush, multiple table versions, tombstones, duplicate IDs, namespace separation, range scans, empty values, locks, large entries, and compaction. One test compares a mixed history against a simple reference map.

`tests/recovery.rs` cuts a small WAL at every possible byte position. It checks that only complete records survive and that appending after recovery works. Other cases reconstruct crashes before/after manifest publication, reject complete CRC failures, and force a manifest I/O error to prove the WAL is retained and the handle is poisoned.

`tests/integrity.rs` changes bytes deliberately. It checks exact-block diagnostics, flips every byte in a small table individually, checks metadata/root coverage, replaces both a table and manifest to test an external checkpoint, and rejects unsafe filenames and inconsistent generations.

`tests/crash_process.rs` starts a real child process, waits until 30 writes are synced, kills the process, and checks recovery and lock release. The `crash_worker` helper is one of the test functions and immediately returns when it is not launched as that child.

`tests/cli.rs` runs the built executable as separate processes. It checks actual command output, JSON, duplicate failure, checkpoint creation, the explicit tamper flag, and nonzero failure status after corruption.

`tests/presentation.rs` sends typed inputs into the actual menu executable. It checks memory and disk traces, reopen recovery, real Merkle levels, invalid input, automatic flush, checkpoints, corruption of a copy, and the isolated edge-case exercises. It also closes input halfway through a form to ensure an unfinished receipt is not saved.

Small unit tests also live at the bottom of the skip-list, Bloom, and Merkle module files. They check those pieces directly. The complete recorded results are in [docs/VALIDATION.md](docs/VALIDATION.md).

Passing these tests is strong evidence for their specific cases. It is not a claim that every hardware failure or malicious filesystem race has been tested.

## 19. Examples, scripts, and the documentation folder

`examples/basic.rs` shows a small Rust client: open the database, record an example event if absent, flush, verify, and print it. This proves the library is usable without the terminal command parser.

`examples/benchmark.rs` writes a chosen number of records, reads them, flushes, compacts, and reports elapsed time. It requires a new directory so old data does not mix with the measurement. It retains that database for inspection. Timings depend on the machine and are not a rigorous benchmark study.

`scripts/demo.sh` builds the program and uses a temporary directory for the entire presentation. It records an event, checks duplicate rejection, demonstrates mutable changes and compaction, creates a checkpoint, deliberately corrupts a table, and requires verification to fail with the expected block diagnostic. It cleans up only its own temporary demo directory.

Every file in `docs/` has a separate purpose:

| File | Read it when you want to know… |
| --- | --- |
| `ORIGINAL_PLAN.txt` | Exactly what the original source plan said |
| `ROADMAP.md` | Which requirements are current and which old instructions were superseded |
| `ARCHITECTURE.md` | How modules communicate and why operations happen in a particular order |
| `FILE_FORMAT.md` | How to interpret every binary field and JSON field on disk |
| `SECURITY.md` | What verification proves, what it cannot prove, and practical limits |
| `TESTING.md` | What is covered and what teammates should investigate next |
| `TEAM_HANDOFF.md` | Suggested review issues, branch commands, and viva answers |
| `VALIDATION.md` | Which checks were actually run locally and their observed results |
| `PRESENTATION.md` | Which menu choices and sample inputs to use during a live under-the-hood demonstration |
| `RUN_AND_IMPLEMENT.md` | How to run the complete demo and implement the proposed next contribution for each teammate |

## 20. Run the project yourself

From the repository:

```bash
cd /home/tharunn/Desktop/3sPADA
cargo build --locked
cargo test --locked --all-targets
bash scripts/demo.sh
```

The demo's duplicate-ID error and final tamper error are expected. The script only prints its final PASS message if those failures occurred correctly.

For a live presentation with prompts and real storage explanations:

```bash
cargo run --locked -- --db presentation-db present
```

Choose 1 to record a receipt, 3 to inspect memory/WAL, 12 to reopen without flushing, 4 to flush, 5 to inspect tables and hashes, 13 to run edge cases separately, and 14 to demonstrate corruption of a copy. Choose 0 to exit. [PRESENTATION.md](docs/PRESENTATION.md) provides a complete walkthrough.

For individual commands against a database you want to keep:

```bash
cargo run -- --db demo-db init
cargo run -- --db demo-db put homework finished
cargo run -- --db demo-db get homework
cargo run -- --db demo-db flush
cargo run -- --db demo-db verify
cargo run -- --db demo-db stats
```

Use `cargo run -- --help` to see commands. The first `--` tells Cargo that the following arguments belong to the program. Use `cargo doc --no-deps` to generate API documentation under `target/doc/`.

## 21. Honest limitations and the teammates' next job

This is a learning project, not a production bank database. It records audit events, uses one exclusive handle, and has no network server, account authorization, balance engine, encryption, signatures, or background workers.

Local roots cannot defeat an attacker who can replace both files and their manifest. External checkpoints need independent protection. A checkpoint describes one exact snapshot, so later legitimate writes or compaction also make it differ. WAL checksums detect accidental damage but do not prevent intentional rewriting or deletion of complete records.

Linux is the tested platform. Tables are capped at 256 MiB, and scans/compaction collect output in memory. Startup verifies every table; this is thorough but can be slow. The code does not automatically repair corrupt tables or clean all crash leftovers. Full power-cut testing and extensive parser fuzzing are not included.

For final teammate review, assign recovery, data semantics, and integrity/demo checks independently. When someone finds a bug, first add a small test reproducing it, then fix the behavior and rerun the checks. [TEAM_HANDOFF.md](docs/TEAM_HANDOFF.md) contains ready-to-copy issue ideas and questions for the presentation. No GitHub issue has been posted and no remote push or merge has been performed.

## 22. Ten-line Rust cheatsheet

1. `struct` groups related fields, like one receipt form with several boxes.
2. `enum` lists possible choices, like `Put` or `Delete`.
3. `impl` attaches useful actions to a type, like teaching a notebook how to open.
4. `Vec<T>` is a growable row of items; `Vec<u8>` is a row of bytes.
5. `Option<T>` means something may be present (`Some`) or absent (`None`).
6. `Result<T, E>` means an operation produced an answer or an error.
7. `?` passes an error back to the caller instead of pretending success.
8. `&T` borrows an item to inspect it; `&mut T` borrows it with permission to change it.
9. `match` chooses what to do for each possible case, like sorting mail by label.
10. `Iterator` provides items one at a time; ownership and `Drop` release resources when their owner ends.
