use itertools::Itertools;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StackVec4 {
    values: [u16; 4],
}

impl StackVec4 {
    pub fn new() -> Self {
        StackVec4 { values: [u16::MAX; 4] }
    }

    pub fn is_empty(&self) -> bool {
        self.values == [u16::MAX; 4]
    }

    pub fn len(&self) -> usize {
        self.values.iter().filter(|&&x| x != u16::MAX).count()
    }

    pub fn count(&self, value: u16) -> usize {
        self.values.iter().filter(|&&x| x == value).count()
    }

    pub fn for_each(&self, mut f: impl FnMut(u16)) {
        if self.is_empty() {
            return;
        }

        for v in self.values {
            if v != u16::MAX {
                f(v)
            }
        }
    }

    pub fn first(&self) -> Option<u16> {
        for v in self.values {
            if v != u16::MAX {
                return Some(v);
            }
        }
        None
    }

    pub fn contains_duplicates(&self) -> bool {
        let dedup_len = self.values.iter().filter(|&&v| v != u16::MAX).dedup().count();
        dedup_len != self.len()
    }
}

impl Index<usize> for StackVec4 {
    type Output = u16;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl IndexMut<usize> for StackVec4 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}
