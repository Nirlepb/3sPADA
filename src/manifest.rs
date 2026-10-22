//! Atomic, durable publication of the active table set.
use crate::{Error, Result, merkle::Hash};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

pub const MANIFEST_FILE: &str = "MANIFEST.json";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableMeta {
    pub file: String,
    pub size: u64,
    pub root: Hash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub generation: u64,
    pub last_seq: u64,
    /// Ordered oldest to newest.
    pub tables: Vec<TableMeta>,
}
impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: 1,
            generation: 0,
            last_seq: 0,
            tables: vec![],
        }
    }
}
impl Manifest {
    pub fn load(dir: &Path) -> Result<Self> {
        Self::read(&dir.join(MANIFEST_FILE))
    }
    pub fn read(path: &Path) -> Result<Self> {
        if fs::metadata(path)?.len() > 16 * 1024 * 1024 {
            return Err(Error::Corruption(
                "manifest/checkpoint exceeds 16 MiB".into(),
            ));
        }
        let manifest: Self = serde_json::from_slice(&fs::read(path)?)?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err(Error::InvalidFormat("unsupported manifest version".into()));
        }
        if (self.generation == 0) != (self.last_seq == 0)
            || (self.generation == 0 && !self.tables.is_empty())
        {
            return Err(Error::Corruption(
                "inconsistent manifest generation/sequence".into(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        let mut previous_generation = 0;
        for table in &self.tables {
            if !valid_table_name(&table.file) || table.size == 0 || !names.insert(&table.file) {
                return Err(Error::Corruption(
                    "invalid or duplicate table filename/size".into(),
                ));
            }
            let generation: u64 = table.file[6..26]
                .parse()
                .map_err(|_| Error::Corruption("table generation overflow".into()))?;
            if generation <= previous_generation || generation > self.generation {
                return Err(Error::Corruption(
                    "table generations are unordered or exceed manifest generation".into(),
                ));
            }
            previous_generation = generation;
        }
        Ok(())
    }
    pub fn commit(&self, dir: &Path) -> Result<()> {
        self.validate()?;
        atomic_write(&dir.join(MANIFEST_FILE), &serde_json::to_vec_pretty(self)?)
    }
    pub fn list_tables(&self) -> &[TableMeta] {
        &self.tables
    }
}
pub fn valid_table_name(name: &str) -> bool {
    name.len() == 30
        && name.starts_with("table-")
        && name.ends_with(".sst")
        && name.as_bytes()[6..26].iter().all(u8::is_ascii_digit)
}
pub fn sync_dir(dir: &Path) -> Result<()> {
    File::open(dir)?.sync_all()?;
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| Error::InvalidInput("path has no filename".into()))?;
    let temp = parent.join(format!(".{}.tmp", name.to_string_lossy()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)?;
    sync_dir(parent)
}

/// The file remains on disk; the OS releases its advisory lock on exit/crash.
pub fn lock(dir: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("LOCK"))?;
    file.try_lock()
        .map_err(|e| Error::Locked(format!("{}: {e}", dir.display())))?;
    Ok(file)
}
