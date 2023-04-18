// TODO move validness checking here

use crate::games::go::util::OptionU16;
use std::collections::HashSet;

// TODO remove options here? we either have both present or neither
// TODO make fields private
// TODO do a circular linked list instead?
//   might yield an easier or more compact implementation
#[derive(Debug, Clone)]
pub struct LinkHead {
    pub first: OptionU16,
    // TODO remove last?
    pub last: OptionU16,
    len: u16,
}

// TODO remove option here, this can be implicit from the containing node
#[derive(Debug, Clone)]
pub struct LinkNode {
    pub prev: OptionU16,
    pub next: OptionU16,
}

// TODO allow multiple implementations for the same type?
//   in theory a value could be part of multiple different linked lists
//   newtypes are not enough since we currently need the storage to have a matching type
pub trait Linked {
    fn link(&self) -> &LinkNode;
    fn link_mut(&mut self) -> &mut LinkNode;
}

impl LinkHead {
    pub fn empty() -> Self {
        Self::full(0)
    }

    /// Build the head for a list containing a single value `index`.
    pub fn single(index: u16) -> Self {
        LinkHead {
            first: OptionU16::Some(index),
            last: OptionU16::Some(index),
            len: 1,
        }
    }

    /// Build the head for a list containing every value in `0..len`.
    pub fn full(len: u16) -> Self {
        if len == 0 {
            LinkHead {
                first: OptionU16::None,
                last: OptionU16::None,
                len: 0,
            }
        } else {
            LinkHead {
                first: OptionU16::Some(0),
                last: OptionU16::Some(len - 1),
                len,
            }
        }
    }

    pub fn pop_front(&mut self, storage: &mut [impl Linked]) -> OptionU16 {
        let first_id = match self.first.to_option() {
            None => return OptionU16::None,
            Some(first_id) => first_id,
        };

        let first = &mut storage[first_id as usize].link_mut();
        let next_id = first.next;

        // update first
        debug_assert_eq!(first.prev, OptionU16::None);
        first.next = OptionU16::None;

        // update head/next
        self.first = next_id;
        self.len -= 1;
        match next_id.to_option() {
            None => self.last = next_id,
            Some(next) => storage[next as usize].link_mut().prev = OptionU16::None,
        }

        OptionU16::Some(first_id)
    }

    pub fn insert_front(&mut self, index: u16, storage: &mut [impl Linked]) {
        let mut other = LinkHead::single(index);
        self.splice_front_take(&mut other, storage);
        debug_assert!(other.is_empty());
    }

    pub fn splice_front_take(&mut self, other: &mut LinkHead, storage: &mut [impl Linked]) {
        // middle connection
        if let Some(other_last) = other.last.to_option() {
            storage[other_last as usize].link_mut().next = self.first;
        }
        if let Some(self_first) = self.first.to_option() {
            storage[self_first as usize].link_mut().prev = other.last;
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

    pub fn remove(&mut self, index: u16, storage: &mut [impl Linked]) {
        let node = storage[index as usize].link_mut();
        let prev = node.prev.take();
        let next = node.next.take();

        match prev.to_option() {
            None => self.first = next,
            Some(prev) => storage[prev as usize].link_mut().next = next,
        }
        match next.to_option() {
            None => self.last = prev,
            Some(next) => storage[next as usize].link_mut().prev = prev,
        }

        self.len -= 1;
    }

    pub fn len(&self) -> u16 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter<'s, L: Linked>(&self, storage: &'s [L]) -> LinkIterator<'s, L> {
        LinkIterator::new(self, storage)
    }

    // TODO implement an iterator for this, similar to LinkIterator
    pub fn for_each_mut<L: Linked>(&self, storage: &mut [L], mut f: impl FnMut(&mut [L], u16)) {
        let mut next = self.first;
        let mut prev = OptionU16::None;
        while let Some(curr) = next.to_option() {
            let link = storage[curr as usize].link();
            debug_assert_eq!(prev, link.prev);
            prev = OptionU16::Some(curr);

            next = link.next;
            f(storage, curr);
        }
    }

    /// Checks whether this list is valid and returns all of the contained nodes.
    /// In the panic messages "A |-> B" is read as "A points to B but B does not point back".
    pub fn assert_valid_and_collect(&self, storage: &[impl Linked]) -> HashSet<u16> {
        let mut seen = HashSet::new();

        match self.first.to_option() {
            None => assert_eq!(OptionU16::None, self.last, "Wrong last: start |-> end"),
            Some(first) => assert_eq!(
                OptionU16::None,
                storage[first as usize].link().prev,
                "Wrong prev: start |-> {}",
                first
            ),
        }

        let mut next = self.first;
        while let Some(curr) = next.to_option() {
            let inserted = seen.insert(curr);
            assert!(inserted, "Empty linked list contains loop including {}", curr);

            next = storage[curr as usize].link().next;
            match next.to_option() {
                None => assert_eq!(OptionU16::Some(curr), self.last, "Wrong last: {} |-> end", curr),
                Some(next) => assert_eq!(
                    OptionU16::Some(curr),
                    storage[next as usize].link().prev,
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
        LinkNode {
            prev: OptionU16::None,
            next: OptionU16::None,
        }
    }

    pub fn full(len: u16, index: u16) -> Self {
        LinkNode {
            prev: if index > 0 {
                OptionU16::Some(index - 1)
            } else {
                OptionU16::None
            },
            next: if index + 1 < len {
                OptionU16::Some(index + 1)
            } else {
                OptionU16::None
            },
        }
    }

    pub fn is_unconnected_or_single(&self) -> bool {
        self.prev.is_none() && self.next.is_none()
    }
}

#[derive(Debug)]
pub struct LinkIterator<'s, L: Linked> {
    storage: &'s [L],
    next: OptionU16,
    items_left: u16,
}

impl<'s, L: Linked> LinkIterator<'s, L> {
    pub fn new(head: &LinkHead, storage: &'s [L]) -> Self {
        Self {
            storage,
            next: head.first,
            items_left: head.len,
        }
    }
}

impl<'s, L: Linked> Iterator for LinkIterator<'s, L> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next.to_option() {
            None => {
                debug_assert_eq!(self.items_left, 0);
                None
            }
            Some(index) => {
                self.next = self.storage[index as usize].link().next;
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
    //             curr = self.storage[curr as usize].next.unwrap();
    //         }
    //         curr
    //     } else {
    //         // walk backwards
    //         // TODO debug this
    //         let mut curr = self.last.unwrap();
    //         for _ in 0..(n - self.items_left as usize - 1) {
    //             curr = self.storage[curr as usize].prev.unwrap();
    //         }
    //         curr
    //     };
    //
    //     self.next = self.storage[item as usize].next;
    //     self.items_left -= n as u16;
    //
    //     Some(item)
    // }
}

impl<'s, L: Linked> ExactSizeIterator for LinkIterator<'s, L> {
    fn len(&self) -> usize {
        self.items_left as usize
    }
}
