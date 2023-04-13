// TODO move validness checking here

use std::collections::HashSet;

// TODO remove options here? we either have both present or neither
// TODO make fields private
#[derive(Debug, Copy, Clone)]
pub struct LinkHead {
    pub first: Option<u16>,
    // TODO remove last?
    pub last: Option<u16>,
    len: u16,
}

// TODO remove option here, this can be implicit from the containing node
#[derive(Debug, Copy, Clone)]
pub struct LinkNode {
    pub prev: Option<u16>,
    pub next: Option<u16>,
}

pub trait NodeStorage {
    fn get_link(&self, index: u16) -> &LinkNode;
}

pub trait NodeStorageMut: NodeStorage {
    fn get_link_mut(&mut self, index: u16) -> &mut LinkNode;
}

impl LinkHead {
    pub fn full(len: u16) -> Self {
        if len == 0 {
            LinkHead {
                first: None,
                last: None,
                len: 0,
            }
        } else {
            LinkHead {
                first: Some(0),
                last: Some(len - 1),
                len,
            }
        }
    }

    pub fn insert_front(&mut self, index: u16, storage: &mut impl NodeStorageMut) {
        let prev_first = self.first;

        let node = storage.get_link_mut(index);
        debug_assert_eq!(None, node.prev);
        debug_assert_eq!(None, node.next);

        self.first = Some(index);
        node.next = prev_first;

        match prev_first {
            None => self.last = Some(index),
            Some(next) => storage.get_link_mut(next).prev = Some(index),
        }

        self.len += 1;
    }

    pub fn remove(&mut self, index: u16, storage: &mut impl NodeStorageMut) {
        let node = storage.get_link_mut(index);
        let prev = node.prev.take();
        let next = node.next.take();

        match prev {
            None => self.first = next,
            Some(prev) => storage.get_link_mut(prev).next = next,
        }
        match next {
            None => self.last = prev,
            Some(next) => storage.get_link_mut(next).prev = prev,
        }

        self.len -= 1;
    }

    pub fn len(&self) -> u16 {
        self.len
    }

    pub fn iter<S: NodeStorage>(&self, storage: S) -> LinkIterator<S> {
        LinkIterator::new(self, storage)
    }

    /// Checks whether this list is valid and returns all of the contained nodes.
    /// In the panic messages "A |-> B" is read as "A points to B but B does not point back".
    pub fn assert_valid_and_collect<S: NodeStorage>(&self, storage: S) -> HashSet<u16> {
        let mut seen = HashSet::new();

        match self.first {
            None => assert_eq!(None, self.last, "Wrong last: start |-> end"),
            Some(first) => assert_eq!(None, storage.get_link(first).prev, "Wrong prev: start |-> {}", first),
        }

        let mut next = self.first;
        while let Some(curr) = next {
            let inserted = seen.insert(curr);
            assert!(inserted, "Empty linked list contains loop including {}", curr);

            next = storage.get_link(curr).next;
            match next {
                None => assert_eq!(Some(curr), self.last, "Wrong last: {} |-> end", curr),
                Some(next) => assert_eq!(
                    Some(curr),
                    storage.get_link(next).prev,
                    "Wrong prev: {} |-> {}",
                    curr,
                    next
                ),
            }
        }

        assert_eq!(seen.len(), self.len as usize);
        seen
    }
}

impl LinkNode {
    pub fn full(len: u16, index: u16) -> Self {
        LinkNode {
            prev: if index > 0 { Some(index - 1) } else { None },
            next: if index + 1 < len { Some(index + 1) } else { None },
        }
    }

    pub fn empty() -> Self {
        LinkNode { prev: None, next: None }
    }
}

#[derive(Debug)]
pub struct LinkIterator<S: NodeStorage> {
    storage: S,
    next: Option<u16>,
    items_left: u16,
}

impl<S: NodeStorage> LinkIterator<S> {
    pub fn new(head: &LinkHead, storage: S) -> Self {
        Self {
            storage,
            next: head.first,
            items_left: head.len,
        }
    }
}

impl<S: NodeStorage> Iterator for LinkIterator<S> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            None => {
                debug_assert_eq!(self.items_left, 0);
                None
            }
            Some(index) => {
                self.next = self.storage.get_link(index).next;
                self.items_left -= 1;
                Some(index)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<S: NodeStorage> ExactSizeIterator for LinkIterator<S> {
    fn len(&self) -> usize {
        self.items_left as usize
    }
}
