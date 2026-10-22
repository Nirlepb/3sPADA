//! A complete bad frame is corruption. Only an incomplete final frame is
//! discarded, and its bytes are removed before another append is allowed.
use crate::{
    Entry, Error, Result,
    codec::{Decoder, encode},
    types::MAX_RECORD,
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub struct Wal {
    file: File,
}
pub struct Replay {
    pub entries: Vec<Entry>,
    pub valid_bytes: u64,
    pub discarded_tail_bytes: u64,
}
impl Wal {
    pub fn open(path: &Path) -> Result<(Self, Replay)> {
        let replay = Self::replay(path)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .append(true)
            .open(path)?;
        if replay.discarded_tail_bytes > 0 {
            file.set_len(replay.valid_bytes)?;
            file.sync_all()?;
        }
        Ok((Self { file }, replay))
    }
    pub fn append(&mut self, entry: &Entry) -> Result<()> {
        let payload = encode(entry)?;
        let mut frame = Vec::with_capacity(payload.len() + 8);
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&payload);
        frame.extend_from_slice(&crc32fast::hash(&frame).to_le_bytes());
        self.file.write_all(&frame)?;
        self.sync()
    }
    pub fn sync(&self) -> Result<()> {
        self.file.sync_all()?;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<()> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.sync()
    }
    pub fn replay(path: &Path) -> Result<Replay> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Replay {
                    entries: vec![],
                    valid_bytes: 0,
                    discarded_tail_bytes: 0,
                });
            }
            Err(e) => return Err(e.into()),
        };
        let size = file.metadata()?.len();
        let mut offset = 0;
        let mut entries = Vec::new();
        let mut last_seq = 0;
        while size - offset >= 4 {
            let mut length = [0; 4];
            file.read_exact(&mut length)?;
            let len = u32::from_le_bytes(length) as usize;
            if !(17..=MAX_RECORD).contains(&len) {
                return Err(Error::Corruption(format!(
                    "WAL invalid record length at byte {offset}"
                )));
            }
            let frame_size = len as u64 + 8;
            if size - offset < frame_size {
                break;
            }
            let mut payload = vec![0; len];
            file.read_exact(&mut payload)?;
            let mut checksum = [0; 4];
            file.read_exact(&mut checksum)?;
            let mut crc = crc32fast::Hasher::new();
            crc.update(&length);
            crc.update(&payload);
            if crc.finalize() != u32::from_le_bytes(checksum) {
                return Err(Error::Corruption(format!(
                    "WAL CRC mismatch at byte {offset}"
                )));
            }
            let mut decoder = Decoder::new(&payload);
            let entry = decoder.entry()?;
            if decoder.remaining() != 0 || entry.seq <= last_seq {
                return Err(Error::Corruption(format!(
                    "WAL trailing data or non-increasing sequence at byte {offset}"
                )));
            }
            last_seq = entry.seq;
            entries.push(entry);
            offset += frame_size;
        }
        Ok(Replay {
            entries,
            valid_bytes: offset,
            discarded_tail_bytes: size - offset,
        })
    }
}
