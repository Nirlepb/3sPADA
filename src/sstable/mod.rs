//! Immutable sorted data blocks, sparse index, Bloom filter, and authenticated
//! metadata. See docs/FILE_FORMAT.md for the byte-for-byte format.
use crate::{
    Entry, Error, Result,
    bloom::Bloom,
    codec::{Decoder, encode},
    manifest::{TableMeta, atomic_write},
    merkle::{self, Hash},
    types::MAX_RECORD,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

const MAGIC: &[u8; 8] = b"MLSMSST1";
const FOOTER: &[u8; 8] = b"MLSMEND1";
const HEADER_LEN: usize = 32;
const FOOTER_LEN: usize = 32;
const MAX_TABLE: u64 = 256 * 1024 * 1024;
pub const BLOCK_TARGET: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    first_key: Vec<u8>,
    offset: u64,
    length: u32,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    index: Vec<Index>,
    bloom: Bloom,
    entries: u64,
}

pub struct SsTable {
    path: PathBuf,
    metadata: Metadata,
    leaves: Vec<Hash>,
    pub meta: TableMeta,
}

#[derive(Debug, Serialize)]
pub struct BlockInspection {
    pub number: usize,
    pub first_key: Vec<u8>,
    pub offset: u64,
    pub payload_bytes: u32,
    pub leaf_hash: String,
    pub entries: Vec<Entry>,
}
#[derive(Debug, Serialize)]
pub struct TableInspection {
    pub file: String,
    pub bytes: u64,
    pub root: String,
    pub bloom_bytes: usize,
    pub blocks: Vec<BlockInspection>,
    /// Header, data blocks, metadata, then the hash-table/footer leaf.
    pub merkle_levels: Vec<Vec<String>>,
}
impl SsTable {
    pub fn build(path: &Path, entries: &[Entry]) -> Result<Self> {
        if entries.is_empty() {
            return Err(Error::InvalidInput("cannot build an empty SSTable".into()));
        }
        if !entries.windows(2).all(|w| w[0].key < w[1].key) {
            return Err(Error::InvalidInput(
                "SSTable keys must be strictly sorted and unique".into(),
            ));
        }
        let mut blocks: Vec<Vec<u8>> = vec![];
        let mut first_keys = vec![];
        let mut current = vec![];
        let mut bloom = Bloom::new(entries.len());
        for entry in entries {
            let bytes = encode(entry)?;
            if !current.is_empty() && current.len() + bytes.len() > BLOCK_TARGET {
                blocks.push(std::mem::take(&mut current));
            }
            if current.is_empty() {
                first_keys.push(entry.key.clone());
            }
            current.extend_from_slice(&bytes);
            bloom.insert(&entry.key);
        }
        if !current.is_empty() {
            blocks.push(current);
        }
        let mut index = Vec::new();
        let mut offset = HEADER_LEN as u64;
        for (block, key) in blocks.iter().zip(first_keys) {
            index.push(Index {
                first_key: key,
                offset,
                length: block.len() as u32,
            });
            offset += 4 + block.len() as u64;
        }
        let metadata = serde_json::to_vec(&Metadata {
            index,
            bloom,
            entries: entries.len() as u64,
        })?;
        let metadata_offset = offset;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(BLOCK_TARGET as u32).to_le_bytes());
        bytes.extend_from_slice(&(blocks.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
        let mut leaves = vec![merkle::hash_leaf(&bytes)];
        for block in &blocks {
            let start = bytes.len();
            bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
            bytes.extend_from_slice(block);
            leaves.push(merkle::hash_leaf(&bytes[start..]));
        }
        bytes.extend_from_slice(&metadata);
        leaves.push(merkle::hash_leaf(&metadata));
        let hashes_offset = bytes.len() as u64;
        for leaf in &leaves {
            bytes.extend_from_slice(leaf);
        }
        bytes.extend_from_slice(FOOTER);
        bytes.extend_from_slice(&metadata_offset.to_le_bytes());
        bytes.extend_from_slice(&hashes_offset.to_le_bytes());
        bytes.extend_from_slice(&(leaves.len() as u64).to_le_bytes());
        leaves.push(merkle::hash_leaf(&bytes[hashes_offset as usize..]));
        let root = merkle::root(&leaves);
        bytes.extend_from_slice(&root);
        if bytes.len() as u64 > MAX_TABLE {
            return Err(Error::InvalidInput(
                "SSTable exceeds educational 256 MiB limit".into(),
            ));
        }
        atomic_write(path, &bytes)?;
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::InvalidInput("invalid table filename".into()))?
            .to_owned();
        Self::open(
            path,
            &TableMeta {
                file,
                size: bytes.len() as u64,
                root,
            },
        )
    }

    pub fn open(path: &Path, expected: &TableMeta) -> Result<Self> {
        let context = |s: String| Error::TamperDetected(format!("{}: {s}", path.display()));
        let size = fs::metadata(path)?.len();
        if size != expected.size || !(128..=MAX_TABLE).contains(&size) {
            return Err(context("file size mismatch or out of bounds".into()));
        }
        let bytes = fs::read(path)?;
        let root_start = bytes.len() - 32;
        let footer_start = root_start - FOOTER_LEN;
        let mut footer = Decoder::new(&bytes[footer_start..root_start]);
        if footer.take(8)? != FOOTER {
            return Err(context("footer magic mismatch".into()));
        }
        let metadata_offset = usize::try_from(footer.u64()?)
            .map_err(|_| context("metadata offset overflow".into()))?;
        let hashes_offset =
            usize::try_from(footer.u64()?).map_err(|_| context("hash offset overflow".into()))?;
        let count =
            usize::try_from(footer.u64()?).map_err(|_| context("hash count overflow".into()))?;
        if metadata_offset < HEADER_LEN
            || metadata_offset > hashes_offset
            || hashes_offset > footer_start
            || count < 3
            || count > footer_start / 32
            || footer_start - hashes_offset != count * 32
        {
            return Err(context("invalid footer offsets or leaf count".into()));
        }
        let mut saved_leaves = vec![];
        for chunk in bytes[hashes_offset..footer_start].as_chunks::<32>().0 {
            let mut h = [0; 32];
            h.copy_from_slice(chunk);
            saved_leaves.push(h);
        }
        let mut header = Decoder::new(&bytes[..HEADER_LEN]);
        if merkle::hash_leaf(&bytes[..HEADER_LEN]) != saved_leaves[0] {
            return Err(context("header hash mismatch".into()));
        }
        if header.take(8)? != MAGIC || header.u32()? != 1 || header.u32()? != BLOCK_TARGET as u32 {
            return Err(context("unsupported SSTable header".into()));
        }
        let block_count = header.u32()? as usize;
        if header.u32()? != 0
            || header.u64()? != (hashes_offset - metadata_offset) as u64
            || block_count != count - 2
        {
            return Err(context("header count/length mismatch".into()));
        }
        let mut cursor = HEADER_LEN;
        let mut actual_index = vec![];
        let mut total_entries = 0u64;
        let mut previous: Option<Vec<u8>> = None;
        for block in 0..block_count {
            if cursor + 4 > metadata_offset {
                return Err(context(format!("data block {block}: missing length")));
            }
            let mut decoder = Decoder::new(&bytes[cursor..metadata_offset]);
            let len = decoder.u32()? as usize;
            if len == 0 || len > MAX_RECORD.max(BLOCK_TARGET) || len > metadata_offset - cursor - 4
            {
                return Err(context(format!("data block {block}: invalid length")));
            }
            let end = cursor + 4 + len;
            if merkle::hash_leaf(&bytes[cursor..end]) != saved_leaves[block + 1] {
                return Err(context(format!("data block {block}: hash mismatch")));
            }
            let entries = decode_block(&bytes[cursor + 4..end])?;
            for entry in &entries {
                if previous.as_ref().is_some_and(|p| *p >= entry.key) {
                    return Err(context(format!("data block {block}: unordered keys")));
                }
                previous = Some(entry.key.clone());
                total_entries += 1;
            }
            let first = entries
                .first()
                .ok_or_else(|| context(format!("data block {block}: empty")))?;
            actual_index.push(Index {
                first_key: first.key.clone(),
                offset: cursor as u64,
                length: len as u32,
            });
            cursor = end;
        }
        if cursor != metadata_offset {
            return Err(context("unexpected bytes before metadata".into()));
        }
        if merkle::hash_leaf(&bytes[metadata_offset..hashes_offset]) != saved_leaves[count - 1] {
            return Err(context("index/Bloom metadata hash mismatch".into()));
        }
        let mut all_leaves = saved_leaves.clone();
        all_leaves.push(merkle::hash_leaf(&bytes[hashes_offset..root_start]));
        let root = merkle::root(&all_leaves);
        if root != expected.root || bytes[root_start..] != root {
            return Err(context(
                "Merkle root differs from manifest or stored root".into(),
            ));
        }
        let metadata: Metadata = serde_json::from_slice(&bytes[metadata_offset..hashes_offset])?;
        if !metadata.bloom.valid()
            || metadata.entries != total_entries
            || metadata.index.len() != actual_index.len()
            || !metadata.index.iter().zip(&actual_index).all(|(a, b)| {
                a.first_key == b.first_key && a.offset == b.offset && a.length == b.length
            })
        {
            return Err(context(
                "invalid sparse index, Bloom filter, or entry count".into(),
            ));
        }
        // Check the filter has no false negatives before it is used to skip reads.
        for item in &actual_index {
            let start = item.offset as usize + 4;
            for entry in decode_block(&bytes[start..start + item.length as usize])? {
                if !metadata.bloom.contains(&entry.key) {
                    return Err(context("Bloom filter false negative".into()));
                }
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
            metadata,
            leaves: saved_leaves,
            meta: expected.clone(),
        })
    }

    fn read_block(&self, block: usize) -> Result<Vec<Entry>> {
        let item = &self.metadata.index[block];
        let mut file = File::open(&self.path)?;
        if file.metadata()?.len() != self.meta.size {
            return Err(Error::TamperDetected(format!(
                "{}: size changed",
                self.path.display()
            )));
        }
        file.seek(SeekFrom::Start(item.offset))?;
        let mut bytes = vec![0; item.length as usize + 4];
        file.read_exact(&mut bytes)?;
        if merkle::hash_leaf(&bytes) != self.leaves[block + 1] {
            return Err(Error::TamperDetected(format!(
                "{}: data block {block}: hash mismatch",
                self.path.display()
            )));
        }
        decode_block(&bytes[4..])
    }
    pub fn get(&self, key: &[u8]) -> Result<Option<Entry>> {
        self.get_with_trace(key, &mut Vec::new())
    }
    /// Performs the same lookup while reporting the decisions actually taken.
    pub fn get_with_trace(&self, key: &[u8], trace: &mut Vec<String>) -> Result<Option<Entry>> {
        if !self.metadata.bloom.contains(key) {
            trace.push(format!(
                "{}: Bloom filter says DEFINITELY ABSENT; no block read",
                self.meta.file
            ));
            return Ok(None);
        }
        trace.push(format!(
            "{}: Bloom filter says MAYBE PRESENT",
            self.meta.file
        ));
        let position = self
            .metadata
            .index
            .partition_point(|item| item.first_key.as_slice() <= key);
        if position == 0 {
            trace.push("Sparse index: key precedes the first block; absent".into());
            return Ok(None);
        }
        trace.push(format!(
            "Sparse index binary search selected data block {} at byte {}",
            position - 1,
            self.metadata.index[position - 1].offset
        ));
        let entries = self.read_block(position - 1)?;
        trace.push(format!(
            "Read block {}; SHA-256 leaf matched the authenticated hash",
            position - 1
        ));
        let found = entries
            .binary_search_by(|entry| entry.key.as_slice().cmp(key))
            .ok()
            .map(|i| entries[i].clone());
        trace.push(match &found {
            Some(entry) => format!(
                "Block binary search: {:?}, sequence {}",
                entry.kind, entry.seq
            ),
            None => "Block binary search: absent (Bloom MAYBE did not guarantee membership)".into(),
        });
        Ok(found)
    }
    pub fn iter(&self) -> impl Iterator<Item = Result<Entry>> + '_ {
        (0..self.metadata.index.len()).flat_map(|block| match self.read_block(block) {
            Ok(entries) => entries.into_iter().map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
        })
    }
    /// Export the complete framed block containing a requested key.
    pub fn inclusion_proof(&self, key: &[u8]) -> Result<Option<crate::proof::TransactionProof>> {
        self.verify()?;
        if self.get(key)?.is_none() {
            return Ok(None);
        }
        let block = self
            .metadata
            .index
            .partition_point(|item| item.first_key.as_slice() <= key)
            - 1;
        let bytes = fs::read(&self.path)?;
        let root_start = bytes.len() - 32;
        let hash_start = root_start - FOOTER_LEN - self.leaves.len() * 32;
        let mut leaves = self.leaves.clone();
        leaves.push(merkle::hash_leaf(&bytes[hash_start..root_start]));
        let item = &self.metadata.index[block];
        let start = item.offset as usize;
        let proof = crate::proof::TransactionProof {
            version: 1,
            table: self.meta.file.clone(),
            framed_block: bytes[start..start + 4 + item.length as usize].to_vec(),
            path: merkle::prove(&leaves, block + 1)
                .ok_or_else(|| Error::InvalidInput("invalid proof block".into()))?,
        };
        if !proof.path.verify(&proof.framed_block, &self.meta.root) {
            return Err(Error::TamperDetected(
                "table changed during proof export".into(),
            ));
        }
        Ok(Some(proof))
    }
    pub fn verify(&self) -> Result<()> {
        Self::open(&self.path, &self.meta).map(|_| ())
    }
    pub fn block_count(&self) -> usize {
        self.metadata.index.len()
    }
    pub fn entry_count(&self) -> u64 {
        self.metadata.entries
    }
    /// Verified snapshots for teaching; this does not change the disk format.
    pub fn inspect(&self) -> Result<TableInspection> {
        self.verify()?;
        let bytes = fs::read(&self.path)?;
        let root_start = bytes.len() - 32;
        let hash_start = root_start - FOOTER_LEN - self.leaves.len() * 32;
        let mut level = self.leaves.clone();
        level.push(merkle::hash_leaf(&bytes[hash_start..root_start]));
        let mut merkle_levels = vec![level.iter().map(merkle::hex).collect()];
        while level.len() > 1 {
            level = level
                .chunks(2)
                .map(|pair| merkle::hash_parent(&pair[0], pair.get(1).unwrap_or(&pair[0])))
                .collect();
            merkle_levels.push(level.iter().map(merkle::hex).collect());
        }
        let mut blocks = Vec::new();
        for (number, item) in self.metadata.index.iter().enumerate() {
            blocks.push(BlockInspection {
                number,
                first_key: item.first_key.clone(),
                offset: item.offset,
                payload_bytes: item.length,
                leaf_hash: merkle::hex(&self.leaves[number + 1]),
                entries: self.read_block(number)?,
            });
        }
        Ok(TableInspection {
            file: self.meta.file.clone(),
            bytes: self.meta.size,
            root: merkle::hex(&self.meta.root),
            bloom_bytes: self.metadata.bloom.byte_count(),
            blocks,
            merkle_levels,
        })
    }
    /// Used only by the deliberate tamper demonstration.
    pub fn block_payload_offset(&self, block: usize) -> Result<u64> {
        self.metadata
            .index
            .get(block)
            .map(|i| i.offset + 4)
            .ok_or_else(|| Error::InvalidInput("block number out of range".into()))
    }
}
fn decode_block(bytes: &[u8]) -> Result<Vec<Entry>> {
    let mut decoder = Decoder::new(bytes);
    let mut entries = vec![];
    while decoder.remaining() > 0 {
        entries.push(decoder.entry()?);
    }
    Ok(entries)
}
