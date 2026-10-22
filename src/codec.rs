//! Shared bounded binary encoding. All integers use little endian.
use crate::{Entry, Error, Kind, Result};

pub fn encode(entry: &Entry) -> Result<Vec<u8>> {
    entry.validate()?;
    let mut out = Vec::with_capacity(entry.encoded_size());
    out.extend_from_slice(&entry.seq.to_le_bytes());
    out.push(match entry.kind {
        Kind::Put => 1,
        Kind::Delete => 2,
    });
    out.extend_from_slice(&(entry.key.len() as u32).to_le_bytes());
    out.extend_from_slice(&entry.key);
    out.extend_from_slice(&(entry.value.len() as u32).to_le_bytes());
    out.extend_from_slice(&entry.value);
    Ok(out)
}

pub struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| Error::Corruption("length overflow".into()))?;
        let value = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| Error::Corruption("truncated binary field".into()))?;
        self.pos = end;
        Ok(value)
    }
    pub fn u32(&mut self) -> Result<u32> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }
    pub fn u64(&mut self) -> Result<u64> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(bytes))
    }
    pub fn entry(&mut self) -> Result<Entry> {
        let seq = self.u64()?;
        let kind = match self.take(1)?[0] {
            1 => Kind::Put,
            2 => Kind::Delete,
            _ => return Err(Error::Corruption("unknown entry kind".into())),
        };
        let key_len = self.u32()? as usize;
        if key_len > crate::types::MAX_KEY {
            return Err(Error::Corruption("key too large".into()));
        }
        let key = self.take(key_len)?.to_vec();
        let value_len = self.u32()? as usize;
        if value_len > crate::types::MAX_VALUE {
            return Err(Error::Corruption("value too large".into()));
        }
        let value = self.take(value_len)?.to_vec();
        let entry = Entry {
            key,
            value,
            seq,
            kind,
        };
        entry
            .validate()
            .map_err(|e| Error::Corruption(e.to_string()))?;
        Ok(entry)
    }
}
