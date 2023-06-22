use crate::board::Player;
use std::ops::{Index, IndexMut};

/// Structure to hold a value for each player.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PlayerBox<T> {
    pub a: T,
    pub b: T,
}

/// Structure to hold a value for the pov and non-pov player.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PovBox<T> {
    pub pov: T,
    pub other: T,
}

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

impl<T> PlayerBox<T> {
    pub fn new(a: T, b: T) -> Self {
        Self { a, b }
    }

    pub fn as_ref(&self) -> PlayerBox<&T> {
        PlayerBox { a: &self.a, b: &self.b }
    }

    pub fn as_ref_mut(&mut self) -> PlayerBox<&mut T> {
        PlayerBox {
            a: &mut self.a,
            b: &mut self.b,
        }
    }
}

impl<T> PovBox<T> {
    pub fn new(pov: T, other: T) -> Self {
        Self { pov, other }
    }

    pub fn as_ref(&self) -> PovBox<&T> {
        PovBox {
            pov: &self.pov,
            other: &self.other,
        }
    }

    pub fn as_ref_mut(&mut self) -> PovBox<&mut T> {
        PovBox {
            pov: &mut self.pov,
            other: &mut self.other,
        }
    }
}

impl<T> Pov for PovBox<T> {
    type Output = PlayerBox<T>;

    fn un_pov(self, pov: Player) -> Self::Output {
        match pov {
            Player::A => PlayerBox {
                a: self.pov,
                b: self.other,
            },
            Player::B => PlayerBox {
                a: self.other,
                b: self.pov,
            },
        }
    }
}

impl<T> NonPov for PlayerBox<T> {
    type Output = PovBox<T>;

    fn pov(self, pov: Player) -> Self::Output {
        match pov {
            Player::A => PovBox {
                pov: self.a,
                other: self.b,
            },
            Player::B => PovBox {
                pov: self.b,
                other: self.a,
            },
        }
    }
}

impl<T> Index<Player> for PlayerBox<T> {
    type Output = T;

    fn index(&self, index: Player) -> &Self::Output {
        match index {
            Player::A => &self.a,
            Player::B => &self.b,
        }
    }
}

impl<T> IndexMut<Player> for PlayerBox<T> {
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        match index {
            Player::A => &mut self.a,
            Player::B => &mut self.b,
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
