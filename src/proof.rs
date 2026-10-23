//! Transaction membership in one authenticated SSTable block of a trusted snapshot.
use crate::{
    Error, Kind, Result, Transaction, codec::Decoder, manifest::Manifest, merkle::InclusionProof,
};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionProof {
    pub version: u32,
    pub table: String,
    pub framed_block: Vec<u8>,
    pub path: InclusionProof,
}
impl TransactionProof {
    /// The checkpoint must be obtained independently of the proof sender.
    pub fn verify(&self, id: &str, checkpoint: &Manifest) -> Result<Transaction> {
        checkpoint.validate()?;
        let fail = || Error::TamperDetected("invalid transaction inclusion proof".into());
        let table = checkpoint
            .tables
            .iter()
            .find(|table| table.file == self.table)
            .ok_or_else(fail)?;
        // Header is leaf 0, and metadata/footer occupy the final two leaves.
        if self.version != 1
            || self.path.leaf_count < 4
            || self.path.leaf_index == 0
            || self.path.leaf_index >= self.path.leaf_count - 2
            || !self.path.verify(&self.framed_block, &table.root)
        {
            return Err(fail());
        }
        let mut frame = Decoder::new(&self.framed_block);
        let length = frame.u32()? as usize;
        if length != frame.remaining() {
            return Err(fail());
        }
        let mut block = Decoder::new(frame.take(length)?);
        let mut expected_key = b"t:".to_vec();
        expected_key.extend_from_slice(id.as_bytes());
        let mut found = None;
        let mut previous: Option<Vec<u8>> = None;
        while block.remaining() > 0 {
            let entry = block.entry()?;
            if previous.as_ref().is_some_and(|key| *key >= entry.key)
                || entry.seq > checkpoint.last_seq
            {
                return Err(fail());
            }
            previous = Some(entry.key.clone());
            if entry.key == expected_key {
                if entry.kind != Kind::Put {
                    return Err(fail());
                }
                let transaction: Transaction = serde_json::from_slice(&entry.value)?;
                transaction.validate()?;
                if transaction.id != id {
                    return Err(fail());
                }
                found = Some(transaction);
            }
        }
        found.ok_or_else(fail)
    }
}
