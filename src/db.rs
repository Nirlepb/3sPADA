use crate::{
    Entry, Error, Kind, Result, Transaction,
    manifest::{self, MANIFEST_FILE, Manifest},
    memtable::Memtable,
    sstable::SsTable,
    wal::Wal,
};
use serde::Serialize;
use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub memtable_bytes: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            memtable_bytes: 1024 * 1024,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Stats {
    pub generation: u64,
    pub last_sequence: u64,
    pub persisted_sequence: u64,
    pub memtable_entries: usize,
    pub memtable_bytes: usize,
    pub sstables: usize,
    pub sstable_bytes: u64,
    pub data_blocks: usize,
    pub recovered_tail_bytes: u64,
}
pub struct Db {
    dir: PathBuf,
    options: Options,
    manifest: Manifest,
    tables: Vec<SsTable>,
    memtable: Memtable,
    wal: Wal,
    seq: u64,
    poisoned: bool,
    recovered_tail_bytes: u64,
    trace: RefCell<Option<Vec<String>>>,
    _lock: File,
}
impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_options(path, Options::default())
    }
    pub fn open_with_options(path: impl AsRef<Path>, options: Options) -> Result<Self> {
        if options.memtable_bytes == 0 || options.memtable_bytes > 64 * 1024 * 1024 {
            return Err(Error::InvalidInput(
                "memtable threshold must be 1 byte..64 MiB".into(),
            ));
        }
        let dir = path.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        // Persist creation of the database directory before acknowledging data.
        let parent = dir
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        manifest::sync_dir(parent)?;
        let lock = manifest::lock(&dir)?;
        let manifest = if dir.join(MANIFEST_FILE).exists() {
            Manifest::load(&dir)?
        } else {
            // A missing manifest in an existing store must never mean "empty".
            for item in fs::read_dir(&dir)? {
                let name = item?.file_name();
                if name != "LOCK" && name != ".MANIFEST.json.tmp" {
                    return Err(Error::Corruption(
                        "manifest missing from nonempty database directory".into(),
                    ));
                }
            }
            let initial = Manifest::default();
            initial.commit(&dir)?;
            initial
        };
        let mut tables = vec![];
        for table in &manifest.tables {
            tables.push(SsTable::open(&dir.join(&table.file), table)?);
        }
        let (wal, replay) = Wal::open(&dir.join("WAL"))?;
        manifest::sync_dir(&dir)?;
        let mut memtable = Memtable::new();
        let mut seq = manifest.last_seq;
        for entry in replay.entries {
            if entry.seq > manifest.last_seq {
                if entry.seq
                    != seq
                        .checked_add(1)
                        .ok_or_else(|| Error::Corruption("sequence overflow".into()))?
                {
                    return Err(Error::Corruption(
                        "gap in unflushed WAL sequence numbers".into(),
                    ));
                }
                validate_namespace(&entry)?;
                seq = entry.seq;
                memtable.insert(entry);
            }
        }
        let db = Self {
            dir,
            options,
            manifest,
            tables,
            memtable,
            wal,
            seq,
            poisoned: false,
            recovered_tail_bytes: replay.discarded_tail_bytes,
            trace: RefCell::new(None),
            _lock: lock,
        };
        // Validate persisted records, including their namespaces and sequence bounds.
        for table in &db.tables {
            for entry in table.iter() {
                let entry = entry?;
                validate_namespace(&entry)?;
                if entry.seq > db.manifest.last_seq {
                    return Err(Error::Corruption("table sequence exceeds manifest".into()));
                }
            }
        }
        Ok(db)
    }
    fn healthy(&self) -> Result<()> {
        if self.poisoned {
            Err(Error::Poisoned)
        } else {
            Ok(())
        }
    }
    /// Tracing is opt-in; the presentation prints events after the operation.
    pub fn enable_trace(&self) {
        *self.trace.borrow_mut() = Some(Vec::new());
    }
    pub fn take_trace(&self) -> Vec<String> {
        self.trace
            .borrow_mut()
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }
    fn trace(&self, message: impl Into<String>) {
        if let Some(events) = self.trace.borrow_mut().as_mut() {
            events.push(message.into());
        }
    }
    pub fn memtable_entries(&self) -> Result<Vec<Entry>> {
        self.healthy()?;
        Ok(self.memtable.iter().cloned().collect())
    }
    pub fn memtable_levels(&self) -> Result<Vec<(usize, Vec<Vec<u8>>)>> {
        self.healthy()?;
        Ok(self.memtable.levels())
    }
    pub fn inspect_tables(&self) -> Result<Vec<crate::sstable::TableInspection>> {
        self.healthy()?;
        self.tables.iter().map(SsTable::inspect).collect()
    }
    fn lookup(&self, key: &[u8]) -> Result<Option<Entry>> {
        self.healthy()?;
        self.trace(format!(
            "Search skip-list memtable for {:?}",
            String::from_utf8_lossy(key)
        ));
        if let Some(entry) = self.memtable.get(key) {
            self.trace(format!(
                "Memtable HIT: {:?}, sequence {}; no SSTable lookup needed",
                entry.kind, entry.seq
            ));
            return Ok(Some(entry.clone()));
        }
        self.trace("Memtable MISS; inspect SSTables newest first");
        for table in self.tables.iter().rev() {
            let mut events = Vec::new();
            let found = table.get_with_trace(key, &mut events);
            for event in events {
                self.trace(event);
            }
            if let Some(entry) = found? {
                return Ok(Some(entry));
            }
        }
        self.trace("No matching entry in memory or active tables");
        Ok(None)
    }
    fn write(&mut self, key: Vec<u8>, value: Vec<u8>, kind: Kind) -> Result<u64> {
        self.healthy()?;
        let seq = self
            .seq
            .checked_add(1)
            .ok_or_else(|| Error::InvalidInput("sequence exhausted".into()))?;
        let entry = Entry {
            key,
            value,
            seq,
            kind,
        };
        entry.validate()?;
        self.trace(format!("Entry validated; assigned sequence {seq}"));
        if let Err(e) = self.wal.append(&entry) {
            self.poisoned = true;
            return Err(e);
        }
        self.trace(format!(
            "WAL appended length + entry + CRC32, then sync_all completed ({} bytes)",
            entry.encoded_size() + 8
        ));
        self.seq = seq;
        self.memtable.insert(entry);
        self.trace(format!(
            "Skip-list insert/update complete; {} entries, {} estimated encoded bytes",
            self.memtable.len(),
            self.memtable.approx_size()
        ));
        if self.memtable.approx_size() >= self.options.memtable_bytes {
            self.trace(format!(
                "Memtable threshold {} reached; automatic flush",
                self.options.memtable_bytes
            ));
            self.flush()?;
        }
        Ok(seq)
    }
    /// The mutable demonstration key/value namespace cannot overwrite ledger events.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<u64> {
        self.write(namespaced(b'k', key)?, value.to_vec(), Kind::Put)
    }
    pub fn delete(&mut self, key: &[u8]) -> Result<u64> {
        self.write(namespaced(b'k', key)?, vec![], Kind::Delete)
    }
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self
            .lookup(&namespaced(b'k', key)?)?
            .filter(|e| e.kind == Kind::Put)
            .map(|e| e.value))
    }
    /// Append an immutable financial audit event; duplicate IDs are rejected.
    pub fn record(&mut self, transaction: Transaction) -> Result<u64> {
        transaction.validate()?;
        self.trace("Financial fields validated: nonempty ID/accounts, distinct accounts, positive integer paise");
        let key = namespaced(b't', transaction.id.as_bytes())?;
        if self.lookup(&key)?.is_some() {
            self.trace("Duplicate ID found; rejected before any WAL append");
            return Err(Error::DuplicateTransaction(transaction.id));
        }
        self.trace("Transaction ID is new; serialize receipt in the protected t: namespace");
        self.write(key, serde_json::to_vec(&transaction)?, Kind::Put)
    }
    pub fn get_transaction(&self, id: &str) -> Result<Option<Transaction>> {
        self.lookup(&namespaced(b't', id.as_bytes())?)?
            .map(|entry| serde_json::from_slice(&entry.value).map_err(Error::from))
            .transpose()
    }
    fn merged(&self) -> Result<BTreeMap<Vec<u8>, Entry>> {
        self.healthy()?;
        let mut entries: BTreeMap<Vec<u8>, Entry> = BTreeMap::new();
        for table in &self.tables {
            for entry in table.iter() {
                insert_newest(&mut entries, entry?);
            }
        }
        for entry in self.memtable.iter() {
            insert_newest(&mut entries, entry.clone());
        }
        Ok(entries)
    }
    /// Lexicographic byte order, inclusive start and exclusive end.
    pub fn scan(&self, start: Option<&[u8]>, end: Option<&[u8]>) -> Result<Vec<Entry>> {
        if start.zip(end).is_some_and(|(a, b)| a > b) {
            return Err(Error::InvalidInput("scan start is after end".into()));
        }
        let mut output = vec![];
        for (_, mut entry) in self.merged()? {
            if !entry.key.starts_with(b"k:") || entry.kind == Kind::Delete {
                continue;
            }
            let key = &entry.key[2..];
            if start.is_none_or(|s| key >= s) && end.is_none_or(|e| key < e) {
                entry.key = key.to_vec();
                output.push(entry);
            }
        }
        Ok(output)
    }
    /// Pending transactions must be flushed explicitly before exporting a proof.
    pub fn transaction_proof(&self, id: &str) -> Result<crate::proof::TransactionProof> {
        self.healthy()?;
        let key = namespaced(b't', id.as_bytes())?;
        if self.memtable.get(&key).is_some() {
            return Err(Error::InvalidInput(
                "transaction is pending; flush first".into(),
            ));
        }
        for table in self.tables.iter().rev() {
            if let Some(proof) = table.inclusion_proof(&key)? {
                return Ok(proof);
            }
        }
        Err(Error::InvalidInput("unknown transaction ID".into()))
    }
    pub fn transactions(&self) -> Result<Vec<Transaction>> {
        self.merged()?
            .into_values()
            .filter(|e| e.key.starts_with(b"t:"))
            .map(|e| serde_json::from_slice(&e.value).map_err(Error::from))
            .collect()
    }
    pub fn flush(&mut self) -> Result<()> {
        self.healthy()?;
        if self.memtable.is_empty() {
            self.trace("Memtable empty; flush has no work and creates no table");
            return Ok(());
        }
        let result = self.flush_inner();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn next_manifest(&self) -> Result<Manifest> {
        let mut next = self.manifest.clone();
        next.generation = next
            .generation
            .checked_add(1)
            .ok_or_else(|| Error::InvalidInput("generation exhausted".into()))?;
        next.last_seq = self.seq;
        Ok(next)
    }
    fn flush_inner(&mut self) -> Result<()> {
        let entries: Vec<_> = self.memtable.iter().cloned().collect();
        let mut next = self.next_manifest()?;
        let path = self.dir.join(format!("table-{:020}.sst", next.generation));
        #[cfg(feature = "fault-injection")]
        crate::fault::hit("flush-before-table")?;
        let table = SsTable::build(&path, &entries)?;
        #[cfg(feature = "fault-injection")]
        crate::fault::hit("flush-after-table")?;
        self.trace(format!("SSTable built from {} sorted entries in {} blocks; index, Bloom filter and Merkle root created; file synced and published: {}", entries.len(), table.block_count(), table.meta.file));
        self.trace(format!(
            "Merkle root: {}",
            crate::merkle::hex(&table.meta.root)
        ));
        next.tables.push(table.meta.clone());
        next.commit(&self.dir)?;
        #[cfg(feature = "fault-injection")]
        crate::fault::hit("flush-after-manifest")?;
        self.trace(format!(
            "Manifest generation {} synced and atomically published",
            next.generation
        ));
        self.manifest = next;
        self.tables.push(table);
        self.memtable = Memtable::new();
        // Only after both table and manifest are durable is the WAL redundant.
        self.wal.reset()?;
        self.trace("WAL truncated and synced AFTER the SSTable and manifest became durable");
        Ok(())
    }
    pub fn compact(&mut self) -> Result<()> {
        self.flush()?;
        if self.tables.is_empty() {
            return Ok(());
        }
        let result = self.compact_inner();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    fn compact_inner(&mut self) -> Result<()> {
        self.trace(format!(
            "Heap merge of all {} active sorted table streams; highest sequence wins per key",
            self.tables.len()
        ));
        // A min-heap chooses the next key among ordered table streams.
        let merged = {
            let mut streams: Vec<_> = self.tables.iter().map(SsTable::iter).collect();
            let mut pending: Vec<Option<Entry>> = vec![None; streams.len()];
            let mut heap = BinaryHeap::new();
            for (i, stream) in streams.iter_mut().enumerate() {
                if let Some(entry) = stream.next() {
                    let entry = entry?;
                    heap.push(Reverse((entry.key.clone(), i)));
                    pending[i] = Some(entry);
                }
            }
            let mut output: Vec<Entry> = vec![];
            while let Some(Reverse((_, i))) = heap.pop() {
                let entry = pending[i]
                    .take()
                    .ok_or_else(|| Error::Corruption("compaction heap lost its entry".into()))?;
                if let Some(last) = output.last_mut().filter(|last| last.key == entry.key) {
                    if entry.seq > last.seq {
                        *last = entry;
                    }
                } else {
                    output.push(entry);
                }
                if let Some(next) = streams[i].next() {
                    let next = next?;
                    heap.push(Reverse((next.key.clone(), i)));
                    pending[i] = Some(next);
                }
            }
            output.retain(|e| e.kind != Kind::Delete);
            output
        };
        let mut next = self.next_manifest()?;
        next.tables.clear();
        let table = if merged.is_empty() {
            None
        } else {
            Some(SsTable::build(
                &self.dir.join(format!("table-{:020}.sst", next.generation)),
                &merged,
            )?)
        };
        if let Some(table) = &table {
            next.tables.push(table.meta.clone());
        }
        next.commit(&self.dir)?;
        self.trace(format!("Compacted output has {} live entries; published manifest generation {} before deleting retired files", merged.len(), next.generation));
        #[cfg(feature = "fault-injection")]
        crate::fault::hit("compact-after-manifest")?;
        let retired = self.manifest.tables.clone();
        self.manifest = next;
        self.tables = table.into_iter().collect();
        for old in retired {
            fs::remove_file(self.dir.join(old.file))?;
        }
        manifest::sync_dir(&self.dir)
    }
    pub fn verify(&self) -> Result<()> {
        self.healthy()?;
        if Manifest::load(&self.dir)? != self.manifest {
            return Err(Error::TamperDetected(
                "manifest changed while database was open".into(),
            ));
        }
        for table in &self.tables {
            table.verify()?;
            self.trace(format!("Verified {}: all blocks, index, Bloom metadata, hash list, footer, root and expected size", table.meta.file));
        }
        // CRC verifies accidental WAL damage; it is not a cryptographic anchor.
        let replay = Wal::replay(&self.dir.join("WAL"))?;
        if replay.discarded_tail_bytes != 0 {
            return Err(Error::Corruption(
                "incomplete WAL tail detected during verify".into(),
            ));
        }
        self.trace(format!(
            "Verified {} complete WAL frames with CRC32; CRC is not malicious-edit authentication",
            replay.entries.len()
        ));
        Ok(())
    }
    /// Flush first, then export a snapshot of the active roots. Keep it externally.
    pub fn checkpoint(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent = fs::canonicalize(parent)?;
        let dir = fs::canonicalize(&self.dir)?;
        if parent.starts_with(&dir) {
            return Err(Error::InvalidInput(
                "save the trusted checkpoint outside the database directory".into(),
            ));
        }
        if path.exists() {
            return Err(Error::InvalidInput(
                "checkpoint already exists; choose a new filename".into(),
            ));
        }
        self.flush()?;
        self.verify()?;
        // create_new protects an existing trusted checkpoint from being overwritten.
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(&serde_json::to_vec_pretty(&self.manifest)?)?;
        file.sync_all()?;
        manifest::sync_dir(&parent)
    }
    pub fn verify_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.verify()?;
        let checkpoint = Manifest::read(path.as_ref())?;
        if checkpoint != self.manifest || !self.memtable.is_empty() {
            return Err(Error::TamperDetected("database differs from trusted checkpoint (a legitimate later write/flush/compaction also changes the snapshot)".into()));
        }
        Ok(())
    }
    pub fn stats(&self) -> Stats {
        Stats {
            generation: self.manifest.generation,
            last_sequence: self.seq,
            persisted_sequence: self.manifest.last_seq,
            memtable_entries: self.memtable.len(),
            memtable_bytes: self.memtable.approx_size(),
            sstables: self.tables.len(),
            sstable_bytes: self.manifest.tables.iter().map(|t| t.size).sum(),
            data_blocks: self.tables.iter().map(SsTable::block_count).sum(),
            recovered_tail_bytes: self.recovered_tail_bytes,
        }
    }
}
fn namespaced(prefix: u8, key: &[u8]) -> Result<Vec<u8>> {
    if key.is_empty() || key.len() > crate::types::MAX_KEY - 2 {
        return Err(Error::InvalidInput("key must be 1..65534 bytes".into()));
    }
    let mut result = vec![prefix, b':'];
    result.extend_from_slice(key);
    Ok(result)
}
fn insert_newest(entries: &mut BTreeMap<Vec<u8>, Entry>, entry: Entry) {
    if entries
        .get(&entry.key)
        .is_none_or(|old| entry.seq > old.seq)
    {
        entries.insert(entry.key.clone(), entry);
    }
}
fn validate_namespace(entry: &Entry) -> Result<()> {
    if entry.key.len() <= 2 {
        return Err(Error::Corruption("empty namespaced key".into()));
    }
    if entry.key.starts_with(b"t:") {
        let transaction: Transaction = serde_json::from_slice(&entry.value)?;
        transaction
            .validate()
            .map_err(|e| Error::Corruption(e.to_string()))?;
        if entry.kind != Kind::Put || transaction.id.as_bytes() != &entry.key[2..] {
            return Err(Error::Corruption("invalid ledger entry".into()));
        }
    } else if !entry.key.starts_with(b"k:") {
        return Err(Error::Corruption("unknown key namespace".into()));
    }
    Ok(())
}
