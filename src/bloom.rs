use crate::merkle::hash_leaf;
use serde::{Deserialize, Serialize};

/// Seven independent 32-bit slices of SHA-256 select bits. False positives
/// are allowed; every present key must return true.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bloom {
    bits: Vec<u8>,
}
impl Bloom {
    pub fn new(keys: usize) -> Self {
        Self {
            bits: vec![0; keys.saturating_mul(10).div_ceil(8).max(8)],
        }
    }
    fn positions(&self, key: &[u8]) -> Vec<usize> {
        let hash = hash_leaf(key);
        hash[..28]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize % (self.bits.len() * 8))
            .collect()
    }
    pub fn insert(&mut self, key: &[u8]) {
        for bit in self.positions(key) {
            self.bits[bit / 8] |= 1 << (bit % 8);
        }
    }
    pub fn contains(&self, key: &[u8]) -> bool {
        if self.bits.is_empty() {
            return true;
        }
        self.positions(key)
            .into_iter()
            .all(|bit| self.bits[bit / 8] & (1 << (bit % 8)) != 0)
    }
    pub fn valid(&self) -> bool {
        !self.bits.is_empty()
    }
    pub fn byte_count(&self) -> usize {
        self.bits.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_false_negatives() {
        let mut bloom = Bloom::new(1000);
        for i in 0u32..1000 {
            bloom.insert(&i.to_le_bytes());
        }
        for i in 0u32..1000 {
            assert!(bloom.contains(&i.to_le_bytes()));
        }
        assert!(!Bloom::new(1).contains(b"absent"));
    }
}
