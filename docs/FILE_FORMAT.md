# Version 1 file formats and API contract

These are the implemented formats. They replace conflicting draft layouts in the original plan. Incompatible changes need a new version and migration strategy.

## Basic choices

| Choice | Value | Reason |
| --- | --- | --- |
| Byte order | Little endian | Portable encoding independent of native machine order. |
| Lengths | `u32` for key, value, WAL payload, block | Bounded records need no 64-bit length. |
| Sequences, offsets, sizes, generation | `u64` | Ample room, with overflow checks. |
| Currency | INR paise, positive `i64` | No floating-point rounding. |
| Timestamp | Caller-supplied Unix milliseconds, `u64` | Explicit and deterministic. |
| Hash | 32-byte SHA-256 | Standard digest without system OpenSSL. |
| Block target | 4096 entry bytes | Small reads; a larger single record gets its own block. |
| Table maximum | 256 MiB | Bounds one file's verification load. |
| `Db::get` | `Result<Option<Vec<u8>>>` | Separates failure, absence, and an owned value. |
| `SsTable::get` | `Result<Option<Entry>>` | DB needs sequence and tombstone information. |
| `Memtable::get` | `Option<&Entry>` | Borrows an existing entry without copying. |
| Duplicate transaction ID | Reject before WAL append | Public financial events are append-only. |

Internal keys are at most 65,536 bytes; public key/value keys allow 1..65,534 bytes because a two-byte prefix is added. Values allow at most 1,048,576 bytes. Transaction IDs allow 1..256 bytes and cannot be whitespace-only. Serialized transaction JSON must fit the value limit.

## Entry encoding and namespaces

```text
[sequence: u64]
[kind: u8, 1 = Put, 2 = Delete]
[key_length: u32][key bytes]
[value_length: u32][value bytes]
```

The fixed portion is 17 bytes. Sequence zero is invalid. A tombstone must have an empty value. Keys in a table must be strictly increasing and unique.

- `k:` + user bytes: mutable demonstration values and tombstones.
- `t:` + UTF-8 transaction ID: immutable financial events, only Put entries.

Supplying `t:abc` to `Db::put` writes `k:t:abc`, which cannot overwrite ledger key `t:abc`.

Transaction JSON has `id`, `from`, `to`, `amount_paise`, `timestamp_ms`, and `note`. Missing `note` defaults to an empty string on decode. Accounts must not be whitespace-only and must differ. Hashes cover the exact JSON bytes written; no cross-language canonical JSON convention is required.

## WAL

```text
[payload_length: u32][encoded Entry][CRC32: u32]
```

Payload length is between 17 and 1,114,129. CRC32 covers the four length bytes and the entire entry payload, including sequence. The checksum is encoded little endian and computed by `crc32fast`.

Frames must have increasing sequences. A complete wrong CRC, invalid length, malformed entry, extra payload bytes, or non-increasing sequence is corruption. If a final frame's declared bytes are not physically present, it is a torn tail; opening truncates to the last valid frame. A 1..3-byte trailing length prefix is incomplete too. A fully present invalid length is an error even at EOF.

The WAL has no separate magic header; its version follows the enclosing manifest. Each append calls `sync_all()` before success. Removing whole valid records cannot be detected by CRC alone.

## SSTable layout

Names are `table-` + 20 decimal digits + `.sst`, for example `table-00000000000000000001.sst`. The number is the publication generation.

```text
32-byte header
data block 0: [length: u32][encoded entries]
data block 1: [length: u32][encoded entries]
...
metadata JSON: sparse index, Bloom bits, entry count
saved content leaf hashes: 32 bytes each
32-byte footer
32-byte Merkle root
```

### Header

| Byte offset | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `MLSMSST1` |
| 8 | 4 | Version 1 |
| 12 | 4 | Block target 4096 |
| 16 | 4 | Data block count |
| 20 | 4 | Reserved, must be zero |
| 24 | 8 | Metadata JSON byte length |

Data blocks contain whole entries; an entry is never split. The length excludes its own four bytes. One entry larger than 4096 bytes receives its own block, up to the entry limit.

### Metadata JSON

```json
{
  "index": [{"first_key": [107, 58, 97], "offset": 32, "length": 25}],
  "bloom": {"bits": [0, 0, 0, 0, 0, 0, 0, 0]},
  "entries": 1
}
```

This shows the shape only; zero Bloom bits are not a valid filter for a populated table. `offset` points to a block's length prefix; `length` counts payload bytes. Keys and Bloom bytes are JSON arrays. The reader compares the index against actual blocks, checks entry count, and verifies the filter accepts every stored key.

The Bloom filter uses about ten bits per entry, with at least eight bytes allocated. Seven 32-bit little-endian slices from `SHA-256(0x00 || key)` choose bits modulo the total bit count. Within a byte, bit zero is least significant. False positives are allowed; false negatives are not.

### Footer

The footer occupies the 32 bytes immediately before the root.

| Offset in footer | Length | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `MLSMEND1` |
| 8 | 8 | Absolute metadata offset |
| 16 | 8 | Absolute saved-hash-table offset |
| 24 | 8 | Saved content leaf count |

For B data blocks, there are B+2 saved content leaves: header, B framed blocks, metadata. The hash region contains exactly `(B+2)*32` bytes and ends at the footer. Counts and offsets are bounded before using them.

### Merkle construction without a cycle

1. Leaf hash is `SHA-256(0x00 || bytes)`.
2. Parent hash is `SHA-256(0x01 || left_hash || right_hash)`.
3. Content leaves cover, in order: complete header; each data block including its length prefix; exact metadata bytes.
4. Save these content hashes to the file.
5. Compute one extra leaf from the exact saved-hash-table bytes **plus the footer**. Do not store this extra leaf in the hash table.
6. Build the root from content leaves followed by that extra leaf. Duplicate the last node when a level has an odd count. A one-leaf tree returns that leaf. An empty tree returns the hash of an empty leaf, although SSTables cannot be empty.
7. Append the root. On opening, compare it with both the recomputed root and the expected root in the manifest.

Every byte before the root is covered, including saved hashes and offsets. The root field is excluded from its own calculation but checked by comparison. No cycle exists: the saved table contains earlier content hashes, not its own hash.

Leaf comparisons localize changed blocks. Damaged framing or footer bytes can instead produce a structural diagnostic. Changing a block and its saved hash still fails the root check against a trusted expected root.

## Manifest and checkpoint JSON

Both `MANIFEST.json` and exported checkpoints have this schema:

```json
{
  "version": 1,
  "generation": 7,
  "last_seq": 42,
  "tables": [{
    "file": "table-00000000000000000007.sst",
    "size": 8192,
    "root": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
  }]
}
```

These example values do not describe an actual valid table. Roots are arrays of exactly 32 byte values, not hexadecimal strings. Tables are ordered oldest to newest. `last_seq` is the highest published sequence, even if compaction removed its tombstone. New databases start with generation 0, last_seq 0, and no tables. JSON files larger than 16 MiB are rejected.

Manifest publication uses `.MANIFEST.json.tmp`, file sync, atomic rename, and directory sync. Table temporary names use `.table-....sst.tmp`. Readers trust only manifest-listed final names. Restricted table basenames prevent path traversal.

Table generation numbers must be positive, strictly increasing in manifest order, and no greater than the manifest generation. Generation zero requires sequence zero and an empty table list; a positive generation requires a positive persisted sequence. These structural checks prevent accidental reuse of active table names.

Checkpoint export uses `create_new` and sync, so it cannot overwrite an existing checkpoint. Export is not an atomic rename: failed or interrupted export can leave a partial output that must not be trusted. Choose a new filename when retrying. External storage controls, not this program's file permissions, protect a trusted checkpoint.
