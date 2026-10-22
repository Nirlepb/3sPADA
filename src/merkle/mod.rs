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
