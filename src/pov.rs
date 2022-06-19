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

impl<V: std::ops::Add<V, Output = V>> std::ops::Add<ScalarAbs<V>> for ScalarAbs<V> {
    type Output = ScalarAbs<V>;

    fn add(self, rhs: ScalarAbs<V>) -> Self::Output {
        ScalarAbs::new(self.value_a + rhs.value_a)
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> std::ops::Sub<ScalarAbs<V>> for ScalarAbs<V> {
    type Output = ScalarAbs<V>;

    fn sub(self, rhs: ScalarAbs<V>) -> Self::Output {
        ScalarAbs::new(self.value_a - rhs.value_a)
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> std::ops::AddAssign<ScalarAbs<V>> for ScalarAbs<V> {
    fn add_assign(&mut self, rhs: ScalarAbs<V>) {
        *self = *self + rhs;
    }
}

impl<V: Copy + std::ops::Mul<V, Output = V>> std::ops::Mul<V> for ScalarAbs<V> {
    type Output = ScalarAbs<V>;

    fn mul(self, rhs: V) -> Self::Output {
        ScalarAbs::new(self.value_a * rhs)
    }
}

impl<V: Copy + std::ops::Div<V, Output = V>> std::ops::Div<V> for ScalarAbs<V> {
    type Output = ScalarAbs<V>;

    fn div(self, rhs: V) -> Self::Output {
        ScalarAbs::new(self.value_a / rhs)
    }
}

impl<V: std::ops::Add<V, Output = V>> std::ops::Add<ScalarPov<V>> for ScalarPov<V> {
    type Output = ScalarPov<V>;

    fn add(self, rhs: ScalarPov<V>) -> Self::Output {
        ScalarPov::new(self.value + rhs.value)
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> std::ops::Sub<ScalarPov<V>> for ScalarPov<V> {
    type Output = ScalarPov<V>;

    fn sub(self, rhs: ScalarPov<V>) -> Self::Output {
        ScalarPov::new(self.value - rhs.value)
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> std::ops::AddAssign<ScalarPov<V>> for ScalarPov<V> {
    fn add_assign(&mut self, rhs: ScalarPov<V>) {
        *self = *self + rhs;
    }
}

impl<V: Copy + std::ops::Mul<V, Output = V>> std::ops::Mul<V> for ScalarPov<V> {
    type Output = ScalarPov<V>;

    fn mul(self, rhs: V) -> Self::Output {
        ScalarPov::new(self.value * rhs)
    }
}

impl<V: Copy + std::ops::Div<V, Output = V>> std::ops::Div<V> for ScalarPov<V> {
    type Output = ScalarPov<V>;

    fn div(self, rhs: V) -> Self::Output {
        ScalarPov::new(self.value / rhs)
    }
}
