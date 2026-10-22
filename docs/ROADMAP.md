# Current project specification

## Source and scope

The supplied file was `/home/tharunn/Desktop/DSA_plan/Plan`, a UTF-8 text file without an extension. `Plan.txt` and a PDF were not present. Its exact contents are preserved as [ORIGINAL_PLAN.txt](ORIGINAL_PLAN.txt). The repository initially contained only a README and Git history, with no Cargo crate or implementation to reuse.

The current request is to implement the complete Rust project, with Tharunn owning all implementation and teammates doing final edge-case debugging. This supersedes the original inspect-only stopping point, stub-only foundation stage, and A/B/C/D implementation ownership restrictions.

That arrangement produced the working baseline. The subsequent contribution discussion proposes substantial new implementation ownership: Nirlep on bounded range scans, Reshvanth on controlled failure/recovery verification, Vijay on Merkle inclusion proofs, and Tharunn on integration. These are planned extensions, not claims that the features or teammate contributions already exist. See [RUN_AND_IMPLEMENT.md](RUN_AND_IMPLEMENT.md) for their contracts and demo plans.

## Implemented requirements

- One library and CLI crate, named `merklelsm`, using synchronous safe Rust.
- Shared `Entry`, `Kind`, `Transaction`, `Error`, and `Result` types.
- A real index-linked skip list in a `Vec`, sorted iteration, and an encoded-size estimate.
- WAL framing, CRC32, sync-before-acknowledgment, replay, torn-tail truncation, and increasing sequence numbers.
- Atomic manifest replacement and table publication; clear the WAL only after both are durable.
- Approximately 4 KiB data blocks, sparse index, Bloom filter, immutable SSTables, block reads, and ordered iteration.
- SHA-256 Merkle roots protecting all SSTable bytes except the root field itself, which is compared to an expected root.
- Error messages identifying a damaged data block when its framing remains readable.
- External trusted checkpoints with table names, sizes, roots, generation, and persisted sequence.
- Duplicate transaction ID rejection, integer paise, and no public update/delete API for ledger events.
- Separate mutable key/value namespace with tombstones, sorted scans, flush, and full heap-based compaction.
- CLI: `init`, `record`, `transaction`, `transactions`, `put`, `get`, `delete`, `scan`, `flush`, `compact`, `verify`, `checkpoint`, `stats`, `tamper`.
- Interactive `present` mode accepts live input and shows actual storage traces, skip-list links, WAL frames, table blocks/index, Bloom decisions, and Merkle levels. It includes isolated edge-case and copy-tamper exercises.
- Tests, CI configuration, a temporary-directory demo, library and timing examples, and beginner documentation.

## Resolved conflicts and explicit choices

| Original ambiguity | Current decision | Reason |
| --- | --- | --- |
| C++/Java tooling in old README | Rust/Cargo | Current request specifies Rust. |
| Balanced tree in README versus skip list in plan | Safe skip list | Detailed DSA guide explicitly requests it. |
| Four people implementing separate modules | Tharunn implements; teammates review | Current contribution plan. |
| Stop after inspection / only produce stubs | Complete implementation | Current task asks for an end-to-end project. |
| Several WAL field orders | `[length][entry payload][CRC32]` | One shared encoder; sequence comes first in the payload. |
| Ignore a final bad CRC versus reject complete bad CRC | Only physically incomplete final frames are discarded | A complete bad checksum is corruption. |
| Mutable keys versus immutable transaction IDs | Separate `k:` and `t:` namespaces | Demonstrates updates without rewriting audit events. |
| Circular Merkle root coverage | Content leaves plus a hash-table/footer leaf | Protects metadata without hashing a root into itself. |
| Bloom filter and compaction were stretch features | Implemented after basic persistence | Supporting DSA components now have integration coverage. |
| Inclusion-proof command | Deferred | Requires another API and test contract beyond full verification. |
| Automatic role branches and foundation commit | Implementation left for user review | Old split applies to a different collaboration model; no push or merge performed. |

## Implementation sequence

1. Inspect the repository and source plan.
2. Freeze types, namespaces, errors, and disk formats.
3. Implement skip list, codec, WAL, Bloom filter, and Merkle functions.
4. Implement SSTable builder/reader and durable manifest publication.
5. Integrate recovery, reads, writes, records, scans, flush, and compaction.
6. Add CLI, external checkpoints, and deliberate corruption demonstration.
7. Validate unit behavior, reference-model equivalence, recovery, process termination, and corruption handling.
8. Explain the project and hand final edge-case review to teammates.

## Completion criteria

The project must compile, pass formatting and Clippy with warnings denied, pass tests, recover acknowledged writes after restart, reject duplicate financial IDs, and reject a deliberately corrupted table. There must be no placeholder implementation or ignored placeholder tests. Teammate review is an additional project phase, not a replacement for implementation-time verification.
