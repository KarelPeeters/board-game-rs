use crate::board::Player;

/// Trait to convert an absolute outcome to a relative one.
pub trait NonPov: Sized {
    type Output: Pov<Output = Self>;

    /// View this outcome from the POV of `pov`.
    fn pov(self, pov: Player) -> Self::Output;

    /// Flip this outcome.
    fn flip(self) -> Self {
        // this is kind of cursed
        self.pov(Player::A).un_pov(Player::B)
    }
}

/// The opposite of [NonPov].
pub trait Pov: Sized {
    type Output: NonPov<Output = Self>;

    /// The opposite of [NonPov::pov];
    fn un_pov(self, pov: Player) -> Self::Output;

    /// Flip this outcome.
    fn flip(self) -> Self {
        // this is kind of cursed
        self.un_pov(Player::A).pov(Player::B)
    }
}

impl<I: NonPov> NonPov for Option<I> {
    type Output = Option<I::Output>;
    fn pov(self, pov: Player) -> Option<I::Output> {
        self.map(|inner| inner.pov(pov))
    }
}

impl<I: Pov> Pov for Option<I> {
    type Output = Option<I::Output>;
    fn un_pov(self, pov: Player) -> Option<I::Output> {
        self.map(|inner| inner.un_pov(pov))
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScalarAbs<V> {
    pub value_a: V,
}

impl<V> ScalarAbs<V> {
    pub fn new(value_a: V) -> Self {
        Self { value_a }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScalarPov<V> {
    pub value: V,
}

impl<V> ScalarPov<V> {
    pub fn new(value: V) -> Self {
        Self { value }
    }
}

impl<V: std::ops::Neg<Output = V>> NonPov for ScalarAbs<V> {
    type Output = ScalarPov<V>;
    fn pov(self, pov: Player) -> Self::Output {
        match pov {
            Player::A => ScalarPov::new(self.value_a),
            Player::B => ScalarPov::new(-self.value_a),
        }
    }
}

impl<V: std::ops::Neg<Output = V>> Pov for ScalarPov<V> {
    type Output = ScalarAbs<V>;
    fn un_pov(self, pov: Player) -> Self::Output {
        match pov {
            Player::A => ScalarAbs::new(self.value),
            Player::B => ScalarAbs::new(-self.value),
        }
    }
}
