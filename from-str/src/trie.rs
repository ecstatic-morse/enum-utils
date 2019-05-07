use std::{cmp::min, iter, mem};
use std::collections::btree_map::{self, BTreeMap, Entry};

type Map<T> = BTreeMap<u8, T>;

#[derive(Debug, Clone)]
pub struct Node<T> {
    bytes: Vec<u8>,
    children: Map<Box<Node<T>>>,
    value: Option<T>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Node {
            bytes: Default::default(),
            children: Default::default(),
            value: None,
        }
    }
}

/// Returns the smallest index where two byte strings are not equal.
fn differs_at(a: &[u8], b: &[u8]) -> Option<usize> {
    // debug_assert_eq!(a.len(), b.len());

    for (i, (&a, &b)) in a.iter().zip(b.iter()).enumerate() {
        if a != b {
            return Some(i);
        }
    }

    None
}

impl<T> Node<T> {
    fn new(bytes: Vec<u8>, value: Option<T>) -> Self {
        Node {
            bytes: bytes.to_owned(),
            value,
            children: Default::default(),
        }
    }

    fn split_at(&mut self, idx: usize) {
        let suffix = self.bytes.split_off(idx);

        let byte = suffix[0];
        let mut child = Box::new(Node::new(suffix, self.value.take()));
        child.children = mem::replace(&mut self.children, Default::default());
        self.children.insert(byte, child);
    }

    pub fn insert(&mut self, bytes: &[u8], value: T) -> Option<T> {
        let l = min(bytes.len(), self.bytes.len());
        let (prefix, mut suffix) = bytes.split_at(l);

        // prefix: "abc"
        // "abd"
        let split_idx = differs_at(prefix, &self.bytes)
            .or_else(|| if l < self.bytes.len() { Some(l) } else { None });

        if let Some(idx) = split_idx {
            self.split_at(idx);
            suffix = &bytes[idx..];
        }

        if suffix.is_empty() {
            return self.value.replace(value);
        }

        match self.children.entry(suffix[0]) {
            Entry::Occupied(mut n) => n.get_mut().insert(suffix, value),
            Entry::Vacant(n) => {
                n.insert(Box::new(Node::new(suffix.to_owned(), Some(value))));
                None
            }
        }
    }

    pub fn get(&self, bytes: &[u8]) -> Option<&T> {
        if bytes.len() < self.bytes.len() {
            return None;
        }

        let (prefix, suffix) = bytes.split_at(self.bytes.len());
        if prefix != &self.bytes[..] {
            return None;
        }

        if suffix.is_empty() {
            return self.value.as_ref();
        }

        self.children
            .get(&suffix[0])
            .map_or(None, |c| c.get(bytes))
    }

    pub fn dfs(&self) -> impl Iterator<Item = (TraversalOrder, NodeRef<'_, T>)> {
        iter::once((TraversalOrder::Pre, self.into()))
            .chain(DfsIter::new(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalOrder {
    Pre,
    Post,
}

#[derive(Debug)]
pub struct NodeRef<'a, T> {
    pub bytes: &'a [u8],
    pub value: Option<&'a T>,
}

impl<'a, T> From<&'a Node<T>> for NodeRef<'a, T> {
    fn from(node: &'a Node<T>) -> Self {
        NodeRef {
            bytes: node.bytes.as_ref(),
            value: node.value.as_ref(),
        }
    }
}

struct DfsIter<'a, T>(Vec<(&'a Node<T>, btree_map::Values<'a, u8, Box<Node<T>>>)>);

impl<'a, T> DfsIter<'a, T> {
    fn new(node: &'a Node<T>) -> Self {
        DfsIter(vec![(node, node.children.values())])
    }
}

impl<'a, T> Iterator for DfsIter<'a, T> {
    type Item = (TraversalOrder, NodeRef<'a, T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (_, children) = self.0.last_mut()?;

        if let Some(node) = children.next() {
            self.0.push((node, node.children.values()));
            Some((TraversalOrder::Pre, NodeRef::from(&**node)))
        } else {
            let (node, _) = self.0.pop().unwrap();
            Some((TraversalOrder::Post, NodeRef::from(&*node)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dfs() {
        use TraversalOrder::*;

        let mut trie = Node::default();
        trie.insert(b"abcd", ());
        trie.insert(b"abcde", ());
        trie.insert(b"abz", ());

        let order: Vec<_> = trie.dfs()
            .map(|(o, n)| (o, n.bytes.as_ref()))
            .collect();

        let expected: Vec<(_, &[u8])> = vec![
            (Pre,  b""),
            (Pre,  b"ab"),
            (Pre,  b"cd"),
            (Pre,  b"e"),
            (Post, b"e"),
            (Post, b"cd"),
            (Pre,  b"z"),
            (Post, b"z"),
            (Post, b"ab"),
            (Post, b""),
        ];

        assert_eq!(order, expected);
    }
}
