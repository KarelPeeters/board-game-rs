use internal_iterator::InternalIterator;

/// Basically [std::ops::ControlFlow] but that hasn't been stabilized yet.
#[derive(Debug)]
pub enum Control<B> {
    Break(B),
    Continue,
}

pub trait InternalIteratorExt: InternalIterator {
    /// Similar to for_each except that the closure can stop the loop while running by returning [Control::Break].
    /// If [Control::Continue] is returned instead the loop just continues.
    /// This is just syntax sugar around [InternalIterator::find_map].
    fn for_each_control<B>(self, mut f: impl FnMut(Self::Item) -> Control<B>) -> Option<B> {
        self.find_map(|x| match f(x) {
            Control::Break(b) => Some(b),
            Control::Continue => None,
        })
    }
}

impl<I: InternalIterator> InternalIteratorExt for I {}
