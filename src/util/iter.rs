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

#[derive(Debug, Clone)]
pub struct PureMap<I, F> {
    inner: I,
    f: F,
}

pub trait IterExt: Iterator {
    /// Pure version of [Iterator::map] that assumes the mapping function does not have side effects.
    /// This means that implemented functions (like [Self::count], [Self::nth], ...) are allowed to skip calling the mapping function if possible.
    /// [Iterator::map] already does this do some extend, but only for a limited set of functions.
    fn pure_map<B, F: Fn(Self::Item) -> B>(self, f: F) -> PureMap<Self, F>
    where
        Self: Sized,
    {
        PureMap { inner: self, f }
    }
}

impl<I: Iterator> IterExt for I {}

impl<I: Iterator, B, F: Fn(I::Item) -> B> Iterator for PureMap<I, F> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(&self.f)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.inner.count()
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.inner.last().map(&self.f)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner.nth(n).map(&self.f)
    }
}

impl<I: ExactSizeIterator, B, F: Fn(I::Item) -> B> ExactSizeIterator for PureMap<I, F> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
