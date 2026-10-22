use crate::{Error, Result};
use serde::{Deserialize, Serialize};

pub const MAX_KEY: usize = 65_536;
pub const MAX_VALUE: usize = 1_048_576;
pub const MAX_RECORD: usize = 17 + MAX_KEY + MAX_VALUE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Kind {
    Put,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub seq: u64,
    pub kind: Kind,
}
impl Entry {
    pub fn validate(&self) -> Result<()> {
        if self.key.is_empty()
            || self.key.len() > MAX_KEY
            || self.value.len() > MAX_VALUE
            || self.seq == 0
        {
            return Err(Error::InvalidInput(
                "invalid entry size, empty key, or zero sequence".into(),
            ));
        }
        if self.kind == Kind::Delete && !self.value.is_empty() {
            return Err(Error::InvalidInput(
                "a tombstone must have an empty value".into(),
            ));
        }
        Ok(())
    }
    pub fn encoded_size(&self) -> usize {
        17 + self.key.len() + self.value.len()
    }
}

/// Amounts are integer INR paise: 1050 means Rs 10.50. This is an audit event,
/// not a balance update or an instruction to transfer money.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount_paise: i64,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub note: String,
}
impl Transaction {
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty()
            || self.id.len() > 256
            || self.from.trim().is_empty()
            || self.to.trim().is_empty()
            || self.amount_paise <= 0
            || self.from == self.to
        {
            return Err(Error::InvalidInput("transaction needs an ID (1..256 bytes), distinct nonempty accounts, and positive integer paise".into()));
        }
        Ok(())
    }
}
