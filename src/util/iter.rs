use std::ops::ControlFlow;

use internal_iterator::InternalIterator;

// TODO replace with default wrapper once https://github.com/jDomantas/internal-iterator/pull/11 is merged
#[derive(Debug, Clone)]
pub struct ClonableInternal<I: Iterator> {
    iter: I,
}

impl<I: Iterator> ClonableInternal<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

impl<I: Iterator> InternalIterator for ClonableInternal<I> {
    type Item = I::Item;

    fn try_for_each<R, F>(mut self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        self.iter.try_for_each(f)
    }

    fn count(self) -> usize {
        self.iter.count()
    }
}
