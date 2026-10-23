//! A safe skip list: Vec owns nodes; links are indices instead of pointers.
use crate::Entry;
const LEVELS: usize = 16;
struct Node {
    entry: Entry,
    next: [Option<usize>; LEVELS],
}
pub struct Memtable {
    nodes: Vec<Node>,
    head: [Option<usize>; LEVELS],
    bytes: usize,
    random: u64,
}
impl Default for Memtable {
    fn default() -> Self {
        Self::new()
    }
}
impl Memtable {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            head: [None; LEVELS],
            bytes: 0,
            random: 0x9e3779b97f4a7c15,
        }
    }
    fn next(&self, node: Option<usize>, level: usize) -> Option<usize> {
        match node {
            Some(i) => self.nodes[i].next[level],
            None => self.head[level],
        }
    }
    fn predecessors(&self, key: &[u8]) -> [Option<usize>; LEVELS] {
        let mut before = [None; LEVELS];
        let mut cursor = None;
        for level in (0..LEVELS).rev() {
            while let Some(i) = self.next(cursor, level) {
                if self.nodes[i].entry.key.as_slice() >= key {
                    break;
                }
                cursor = Some(i);
            }
            before[level] = cursor;
        }
        before
    }
    pub fn insert(&mut self, entry: Entry) {
        let before = self.predecessors(&entry.key);
        if let Some(i) = self.next(before[0], 0)
            && self.nodes[i].entry.key == entry.key
        {
            if entry.seq > self.nodes[i].entry.seq {
                self.bytes = self.bytes - self.nodes[i].entry.encoded_size() + entry.encoded_size();
                self.nodes[i].entry = entry;
            }
            return;
        }
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        let height = (self.random.trailing_zeros() as usize + 1).min(LEVELS);
        let mut next = [None; LEVELS];
        for level in 0..height {
            next[level] = self.next(before[level], level);
        }
        let i = self.nodes.len();
        self.bytes += entry.encoded_size();
        self.nodes.push(Node { entry, next });
        for (level, previous) in before.iter().enumerate().take(height) {
            match previous {
                Some(p) => self.nodes[*p].next[level] = Some(i),
                None => self.head[level] = Some(i),
            }
        }
    }
    pub fn get(&self, key: &[u8]) -> Option<&Entry> {
        let before = self.predecessors(key);
        let node = self.next(before[0], 0)?;
        let entry = &self.nodes[node].entry;
        (entry.key == key).then_some(entry)
    }
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            table: self,
            next: self.head[0],
        }
    }
    /// Seek through skip-list levels, then walk only the requested interval.
    pub fn range<'a>(&'a self, start: &[u8], end: &'a [u8]) -> impl Iterator<Item = &'a Entry> {
        let before = self.predecessors(start);
        Iter {
            table: self,
            next: self.next(before[0], 0),
        }
        .take_while(move |entry| entry.key.as_slice() < end)
    }
    pub fn approx_size(&self) -> usize {
        self.bytes
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    /// Read-only view of the actual forward links, highest occupied level first.
    pub fn levels(&self) -> Vec<(usize, Vec<Vec<u8>>)> {
        let mut levels = Vec::new();
        for level in (0..LEVELS).rev() {
            let mut keys = Vec::new();
            let mut cursor = self.head[level];
            while let Some(i) = cursor {
                keys.push(self.nodes[i].entry.key.clone());
                cursor = self.nodes[i].next[level];
            }
            if !keys.is_empty() {
                levels.push((level, keys));
            }
        }
        levels
    }
}
pub struct Iter<'a> {
    table: &'a Memtable,
    next: Option<usize>,
}
impl<'a> Iterator for Iter<'a> {
    type Item = &'a Entry;
    fn next(&mut self) -> Option<Self::Item> {
        let node = &self.table.nodes[self.next?];
        self.next = node.next[0];
        Some(&node.entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Kind;
    #[test]
    fn ordering_replacement_and_size() {
        let mut table = Memtable::new();
        for i in (0u32..1000).rev() {
            table.insert(Entry {
                key: i.to_be_bytes().to_vec(),
                value: vec![1],
                seq: 1,
                kind: Kind::Put,
            });
        }
        let keys: Vec<_> = table.iter().map(|e| e.key.clone()).collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
        let before = table.approx_size();
        let key = 42u32.to_be_bytes().to_vec();
        table.insert(Entry {
            key: key.clone(),
            value: vec![],
            seq: 2,
            kind: Kind::Delete,
        });
        table.insert(Entry {
            key: key.clone(),
            value: vec![8],
            seq: 1,
            kind: Kind::Put,
        });
        assert_eq!(table.get(&key).expect("present").kind, Kind::Delete);
        assert_eq!(table.approx_size(), before - 1);
        assert_eq!(table.len(), 1000);
        assert!(table.get(b"missing").is_none());
    }
}
