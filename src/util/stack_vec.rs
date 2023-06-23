use std::fmt::{Debug, Formatter};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct StackVec<T, const C: usize> {
    inner: [T; C],
    len: usize,
}

impl<T: Default, const C: usize> Default for StackVec<T, C> {
    fn default() -> Self {
        StackVec {
            inner: [(); C].map(|()| T::default()),
            len: 0,
        }
    }
}

impl<T: Copy, const C: usize> StackVec<T, C> {
    pub fn default_with(padding: T) -> Self {
        StackVec {
            inner: [padding; C],
            len: 0,
        }
    }

    pub fn insert_front(&mut self, value: T) {
        assert!(self.len < self.inner.len());

        self.inner.copy_within(0..self.len, 1);
        self.inner[0] = value;
        self.len += 1;
    }
}

impl<T, const C: usize> StackVec<T, C> {
    pub fn push_back(&mut self, value: T) {
        let index = self.len;
        assert!(index < self.inner.len());

        self.inner[index] = value;
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        (0..self.len).map(move |i| &self[i])
    }
}

impl<T, const C: usize> std::ops::Index<usize> for StackVec<T, C> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len());
        &self.inner[index]
    }
}

impl<T, const C: usize> std::ops::IndexMut<usize> for StackVec<T, C> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len());
        &mut self.inner[index]
    }
}

impl<T: Debug, const C: usize> Debug for StackVec<T, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
