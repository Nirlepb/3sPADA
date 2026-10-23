//! Domain separation prevents confusing a leaf with an internal tree node.
use sha2::{Digest, Sha256};
pub type Hash = [u8; 32];
pub fn hash_leaf(bytes: &[u8]) -> Hash {
    let mut hash = Sha256::new();
    hash.update([0]);
    hash.update(bytes);
    hash.finalize().into()
}
pub fn hash_parent(left: &Hash, right: &Hash) -> Hash {
    let mut hash = Sha256::new();
    hash.update([1]);
    hash.update(left);
    hash.update(right);
    hash.finalize().into()
}
pub fn root(leaves: &[Hash]) -> Hash {
    if leaves.is_empty() {
        return hash_leaf(&[]);
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| hash_parent(&pair[0], pair.get(1).unwrap_or(&pair[0])))
            .collect();
    }
    level[0]
}
pub fn verify(leaves: &[Hash], expected: &Hash) -> bool {
    root(leaves) == *expected
}
pub fn hex(hash: &Hash) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn domain_separation_and_odd_leaves() {
        let a = hash_leaf(b"a");
        let b = hash_leaf(b"b");
        let c = hash_leaf(b"c");
        assert_eq!(
            root(&[a, b, c]),
            hash_parent(&hash_parent(&a, &b), &hash_parent(&c, &c))
        );
        assert_ne!(a, hash_parent(&a, &a));
        assert_eq!(root(&[]), hash_leaf(&[]));
        assert_eq!(
            hex(&hash_leaf(b"")),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
    }
}

/// Sibling ordering follows the index at each level; odd final nodes duplicate themselves.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionProof {
    pub leaf_index: usize,
    pub leaf_count: usize,
    pub siblings: Vec<Hash>,
}
pub fn prove(leaves: &[Hash], index: usize) -> Option<InclusionProof> {
    if index >= leaves.len() {
        return None;
    }
    let mut proof = InclusionProof {
        leaf_index: index,
        leaf_count: leaves.len(),
        siblings: vec![],
    };
    let mut position = index;
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        proof
            .siblings
            .push(level.get(position ^ 1).copied().unwrap_or(level[position]));
        level = level
            .chunks(2)
            .map(|pair| hash_parent(&pair[0], pair.get(1).unwrap_or(&pair[0])))
            .collect();
        position /= 2;
    }
    Some(proof)
}
impl InclusionProof {
    pub fn verify(&self, bytes: &[u8], expected: &Hash) -> bool {
        if self.leaf_count == 0 || self.leaf_index >= self.leaf_count {
            return false;
        }
        let mut node = hash_leaf(bytes);
        let mut position = self.leaf_index;
        let mut width = self.leaf_count;
        let mut siblings = self.siblings.iter();
        while width > 1 {
            let Some(sibling) = siblings.next() else {
                return false;
            };
            if position.is_multiple_of(2) {
                if position == width - 1 && *sibling != node {
                    return false;
                }
                node = hash_parent(&node, sibling);
            } else {
                node = hash_parent(sibling, &node);
            }
            position /= 2;
            width = width.div_ceil(2);
        }
        siblings.next().is_none() && node == *expected
    }
}
