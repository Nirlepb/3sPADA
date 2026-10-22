use merklelsm::{Db, Entry, Kind, Result, wal::Wal};
use std::path::Path;

pub fn stats(db: &Db) {
    let stats = db.stats();
    println!(
        "STATE: sequence {} | persisted {} | generation {} | memory {} entries / {} encoded bytes | {} tables / {} blocks",
        stats.last_sequence,
        stats.persisted_sequence,
        stats.generation,
        stats.memtable_entries,
        stats.memtable_bytes,
        stats.sstables,
        stats.data_blocks
    );
}
pub fn entries(entries: &[Entry]) {
    for entry in entries.iter().take(30) {
        let value = if entry.kind == Kind::Delete {
            "<TOMBSTONE>".into()
        } else {
            preview(&entry.value)
        };
        println!(
            "  key={} | seq={} | {:?} | value={value}",
            preview(&entry.key),
            entry.seq,
            entry.kind
        );
    }
    if entries.len() > 30 {
        println!("  ... showing first 30 of {} entries", entries.len());
    }
}
fn preview(bytes: &[u8]) -> String {
    let shown = &bytes[..bytes.len().min(180)];
    let suffix = if bytes.len() > shown.len() {
        format!(" ... ({} bytes)", bytes.len())
    } else {
        String::new()
    };
    format!("{:?}{suffix}", String::from_utf8_lossy(shown))
}
pub fn memory(db: &Db, path: &Path) -> Result<()> {
    stats(db);
    println!("\nACTUAL SKIP-LIST LINKS (higher levels skip keys):");
    let levels = db.memtable_levels()?;
    if levels.is_empty() {
        println!("  EMPTY; record or put an entry before flushing.");
    }
    for (level, keys) in levels {
        let links = keys
            .iter()
            .take(20)
            .map(|key| preview(key))
            .collect::<Vec<_>>()
            .join(" -> ");
        println!(
            "  Level {level:02}: HEAD -> {links} -> {}",
            if keys.len() > 20 { "..." } else { "END" }
        );
    }
    println!("\nSORTED MEMTABLE ENTRIES (latest version per internal key):");
    entries(&db.memtable_entries()?);
    let replay = Wal::replay(&path.join("WAL"))?;
    println!(
        "\nWAL: {} complete CRC-verified frames; {} valid bytes; {} incomplete tail bytes",
        replay.entries.len(),
        replay.valid_bytes,
        replay.discarded_tail_bytes
    );
    println!("Frame = [length u32][sequence + kind + key/value lengths + bytes][CRC32]");
    let mut offset = 0usize;
    for entry in replay.entries.iter().take(30) {
        println!(
            "  byte {offset}: frame {} bytes; seq {}; {:?}; key={}",
            entry.encoded_size() + 8,
            entry.seq,
            entry.kind,
            preview(&entry.key)
        );
        offset += entry.encoded_size() + 8;
    }
    if replay.entries.len() > 30 {
        println!("  ... showing first 30 frames");
    }
    Ok(())
}
pub fn tables(db: &Db) -> Result<()> {
    let tables = db.inspect_tables()?;
    if tables.is_empty() {
        println!("No SSTables yet. Choose 4 to flush pending entries.");
    }
    for (number, table) in tables.iter().enumerate() {
        println!(
            "\nTABLE {number}: {} | {} bytes | {} blocks\nMerkle root: {}\nBloom filter: {} bytes; 7 bit positions per key; MAYBE requires a real block lookup.",
            table.file,
            table.bytes,
            table.blocks.len(),
            table.root,
            table.bloom_bytes
        );
        println!("SPARSE INDEX AND DATA BLOCKS:");
        for block in table.blocks.iter().take(20) {
            println!(
                "  Block {}: first key {}; offset {}; payload {} bytes; {} entries\n  Leaf hash: {}",
                block.number,
                preview(&block.first_key),
                block.offset,
                block.payload_bytes,
                block.entries.len(),
                block.leaf_hash
            );
            entries(&block.entries);
        }
        if table.blocks.len() > 20 {
            println!("  ... showing first 20 blocks");
        }
        println!(
            "MERKLE LEVELS (full hashes, first 8 nodes per level):\n  Leaves: header, framed data blocks, metadata, hash-list/footer. Odd nodes are duplicated."
        );
        for (level, hashes) in table.merkle_levels.iter().enumerate() {
            println!("  Level {level}: {} nodes", hashes.len());
            for (node, hash) in hashes.iter().take(8).enumerate() {
                println!("    [{node}] {hash}");
            }
            if hashes.len() > 8 {
                println!("    ...");
            }
        }
    }
    Ok(())
}
