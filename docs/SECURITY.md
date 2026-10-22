# Integrity, durability, and limitations

## What is protected

SSTable verification recomputes domain-separated SHA-256 leaf hashes and the Merkle root. It covers data, framing, header, sparse index, Bloom filter, hash table, and footer. Stored roots and file sizes must match the manifest. Reads also authenticate each selected data block against the hashes established when opening the table.

Financial transaction IDs cannot be reused through `Db::record`, and mutable demonstration operations use a separate namespace. This is an API rule, not filesystem write protection. The database is an audit-event store; it does not authorize payments, move funds, or enforce account balances.

## What must be trusted

The local manifest is in the same directory as the data. A person who can replace it and the tables can produce different, internally consistent data. Export a checkpoint after flushing, retain it externally under separate control, and compare the exact snapshot against it. Merely keeping another file beside the database does not establish an independent trust boundary.

Checkpoints are unsigned JSON. They do not prove author identity, attest to a timestamp, or establish consistency between successive snapshots. Later legitimate writes or compaction produce a mismatch too. Exporting a checkpoint now does not retroactively prove records were never changed before that export.

The WAL uses CRC32 for accidental corruption, not authentication. A malicious editor can recompute its CRC, remove complete records, or replace uncheckpointed events. Hash-protected verification applies after table publication; external snapshot comparison applies to an exact flushed state. Encryption, signatures, key management, secure remote anchoring, and append-only consistency proofs are future work.

## Crash guarantees and their boundary

Successful writes have completed a WAL sync. Flush syncs the table and directory, then the manifest and directory, before truncating the WAL. Full compaction publishes the replacement before removing old files. OS locks prevent cooperative processes from concurrently opening the database.

Actual power-loss durability still depends on OS/filesystem/device sync semantics and functioning storage. The automated tests cover process termination and reconstructed crash states, not physical power-cut testing. Advisory locks cannot stop another program deliberately editing files. The database directory is assumed to be controlled by the application; symlink races and a hostile concurrent filesystem operator are outside this prototype's hardening scope.

An I/O failure can leave a write's outcome uncertain, so the handle is poisoned and must be reopened. Query a transaction ID before retrying. A torn WAL tail is truncated on open and reported through statistics/CLI stderr; opening is consequently not a forensic read-only operation. Copy damaged files before investigation if their original bytes matter.

## Practical limits

- Linux is the tested target; Windows directory sync/rename behavior has not been validated.
- One exclusive handle per database; no concurrent readers, server, async runtime, or background compaction.
- Full table verification at startup; long-lived readers retain trusted index/hash metadata.
- SSTables at most 256 MiB, manifest/checkpoint JSON at most 16 MiB, values at most 1 MiB, internal keys at most 64 KiB.
- The fixed skip-list height is 16; its pseudo-random heights are reproducible, not a defense against adversarial key ordering.
- Scans and compaction retain their output in memory. Replaying a large WAL also allocates all recovered entries.
- Memtable byte counts exclude `Vec`, node/link, and allocator overhead. A recovered WAL can exceed the configured flush threshold until a subsequent write or explicit flush.
- A full compaction exceeding the table cap returns an error and requires reopening; there is no automatic split into several output tables.
- Unlisted temporary/orphan files can remain after crashes. They are ignored, not automatically garbage-collected.
- No automatic repair of a corrupt table; verification reports corruption instead of inventing replacement data.
- CLI display is intended for UTF-8 strings; binary library values display with replacement characters.
- No inclusion-proof command, leveled compaction, compression, snapshot reader, or rigorous benchmark framework.

These constraints should be stated in the project demonstration. The project illustrates data structures and integrity mechanisms; it is not a production banking database.
