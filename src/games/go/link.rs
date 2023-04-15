// TODO move validness checking here

use std::collections::HashSet;

// TODO remove options here? we either have both present or neither
// TODO make fields private
// TODO do a circular linked list instead?
//   might yield an easier or more compact implementation
#[derive(Debug, Clone)]
pub struct LinkHead {
    pub first: Option<u16>,
    // TODO remove last?
    pub last: Option<u16>,
    len: u16,
}

// TODO remove option here, this can be implicit from the containing node
#[derive(Debug, Clone)]
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
    pub fn empty() -> Self {
        Self::full(0)
    }

    /// Build the head for a list containing a single value `index`.
    pub fn single(index: u16) -> Self {
        LinkHead {
            first: Some(index),
            last: Some(index),
            len: 1,
        }
    }

    /// Build the head for a list containing every value in `0..len`.
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
        let mut other = LinkHead::single(index);
        self.splice_front_take(&mut other, storage);
        debug_assert!(other.is_empty());
    }

    pub fn splice_front_take(&mut self, other: &mut LinkHead, storage: &mut impl NodeStorageMut) {
        // middle connection
        if let Some(other_last) = other.last {
            storage.get_link_mut(other_last).next = self.first;
        }
        if let Some(self_first) = self.first {
            storage.get_link_mut(self_first).prev = other.last;
        }

        // edges
        let new_head = LinkHead {
            first: other.first.or(self.first),
            last: self.last.or(other.last),
            len: self.len + other.len,
        };

        // results
        *self = new_head;
        *other = LinkHead::empty();
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter<S: NodeStorage>(&self, storage: S) -> LinkIterator<S> {
        LinkIterator::new(self, storage)
    }

    pub fn for_each_mut<S: NodeStorage>(&self, storage: &mut S, mut f: impl FnMut(&mut S, u16)) {
        let mut next = self.first;
        let mut prev = None;
        while let Some(curr) = next {
            let link = storage.get_link(curr);
            debug_assert_eq!(prev, link.prev);
            prev = Some(curr);

            next = link.next;
            f(storage, curr);
        }
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
    pub fn single() -> Self {
        LinkNode { prev: None, next: None }
    }

    pub fn full(len: u16, index: u16) -> Self {
        LinkNode {
            prev: if index > 0 { Some(index - 1) } else { None },
            next: if index + 1 < len { Some(index + 1) } else { None },
        }
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

    // TODO optimize nth?
    // fn nth(&mut self, n: usize) -> Option<Self::Item> {
    //     println!(
    //         "calling nth on self={{next: {:?}, last: {:?}, items_left: {}}}, n={}",
    //         self.next, self.last, self.items_left, n
    //     );
    //
    //     if n >= self.items_left as usize {
    //         self.next = None;
    //         self.items_left = 0;
    //         return None;
    //     }
    //
    //     let item = if n <= (self.items_left / 2) as usize {
    //         // walk forward
    //         let mut curr = self.next.unwrap();
    //         for _ in 0..n {
    //             curr = self.storage.get_link(curr).next.unwrap();
    //         }
    //         curr
    //     } else {
    //         // walk backwards
    //         // TODO debug this
    //         let mut curr = self.last.unwrap();
    //         for _ in 0..(n - self.items_left as usize - 1) {
    //             curr = self.storage.get_link(curr).prev.unwrap();
    //         }
    //         curr
    //     };
    //
    //     self.next = self.storage.get_link(item).next;
    //     self.items_left -= n as u16;
    //
    //     Some(item)
    // }
}

impl<S: NodeStorage> ExactSizeIterator for LinkIterator<S> {
    fn len(&self) -> usize {
        self.items_left as usize
    }
}
