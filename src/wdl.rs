use std::ops::ControlFlow;

use cast_trait::Cast;
use internal_iterator::{InternalIterator, IntoInternalIterator};

use crate::board::{Outcome, Player};
use crate::pov::{NonPov, Pov, ScalarAbs};

/// The outcome of a game from the POV of a certain player. Usually obtained using [Outcome::pov].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum OutcomeWDL {
    Win,
    Draw,
    Loss,
}

/// A collection of [win, draw, loss] values.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WDL<V> {
    pub win: V,
    pub draw: V,
    pub loss: V,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WDLAbs<V> {
    pub win_a: V,
    pub draw: V,
    pub win_b: V,
}

impl Outcome {
    /// Convert this to a [WDLAbs] with a one at the correct place and zero otherwise.
    pub fn to_wdl_abs<V: num_traits::One + Default>(self) -> WDLAbs<V> {
        let mut result = WDLAbs::default();
        *match self {
            Outcome::WonBy(Player::A) => &mut result.win_a,
            Outcome::WonBy(Player::B) => &mut result.win_b,
            Outcome::Draw => &mut result.draw,
        } = V::one();
        result
    }

    /// Convert a win (for a) to `1`, draw to `0` and loss (for a) to `-1`.
    pub fn sign<V: num_traits::Zero + num_traits::One + std::ops::Neg<Output = V>>(self) -> ScalarAbs<V> {
        match self {
            Outcome::WonBy(Player::A) => ScalarAbs::new(V::one()),
            Outcome::Draw => ScalarAbs::new(V::zero()),
            Outcome::WonBy(Player::B) => ScalarAbs::new(-V::one()),
        }
    }
}

impl OutcomeWDL {
    /// Convert this to a [WDL] with a one at the correct place and zero otherwise.
    pub fn to_wdl<V: num_traits::One + Default>(self) -> WDL<V> {
        let mut result = WDL::default();
        *match self {
            OutcomeWDL::Win => &mut result.win,
            OutcomeWDL::Draw => &mut result.draw,
            OutcomeWDL::Loss => &mut result.loss,
        } = V::one();
        result
    }

    /// Convert a win to `1`, draw to `0` and loss to `-1`.
    pub fn sign<V: num_traits::Zero + num_traits::One + std::ops::Neg<Output = V>>(self) -> V {
        match self {
            OutcomeWDL::Win => V::one(),
            OutcomeWDL::Draw => V::zero(),
            OutcomeWDL::Loss => -V::one(),
        }
    }

    /// The reverse of [Outcome::pov].
    pub fn un_pov(self, pov: Player) -> Outcome {
        match self {
            OutcomeWDL::Win => Outcome::WonBy(pov),
            OutcomeWDL::Draw => Outcome::Draw,
            OutcomeWDL::Loss => Outcome::WonBy(pov.other()),
        }
    }

    /// Pick the best possible outcome, assuming `Win > Draw > Loss`.
    /// Make sure to flip the child values as appropriate, this function assumes everything is form the parent POV.
    pub fn best<I: IntoInternalIterator<Item = OutcomeWDL>>(children: I) -> OutcomeWDL {
        Self::best_maybe(children.into_internal_iter().map(Some)).unwrap()
    }

    /// Pick the best possible outcome, assuming `Some(Win) > None > Some(Draw) > Some(Loss)`.
    /// Make sure to flip the child values as appropriate, this function assumes everything is form the parent POV.
    pub fn best_maybe<I: IntoInternalIterator<Item = Option<OutcomeWDL>>>(children: I) -> Option<OutcomeWDL> {
        let mut any_unknown = false;
        let mut all_known_are_loss = true;

        let control = children.into_internal_iter().try_for_each(|child| {
            match child {
                None => {
                    any_unknown = true;
                }
                Some(OutcomeWDL::Win) => {
                    return ControlFlow::Break(());
                }
                Some(OutcomeWDL::Draw) => {
                    all_known_are_loss = false;
                }
                Some(OutcomeWDL::Loss) => {}
            }

            ControlFlow::Continue(())
        });

        if let ControlFlow::Break(()) = control {
            Some(OutcomeWDL::Win)
        } else if any_unknown {
            None
        } else if all_known_are_loss {
            Some(OutcomeWDL::Loss)
        } else {
            Some(OutcomeWDL::Draw)
        }
    }
}

impl<V> NonPov for WDLAbs<V> {
    type Output = WDL<V>;

    fn pov(self, pov: Player) -> WDL<V> {
        let (win, loss) = match pov {
            Player::A => (self.win_a, self.win_b),
            Player::B => (self.win_b, self.win_a),
        };

        WDL {
            win,
            draw: self.draw,
            loss,
        }
    }
}

impl<V> Pov for WDL<V> {
    type Output = WDLAbs<V>;

    fn un_pov(self, pov: Player) -> Self::Output {
        let (win_a, win_b) = match pov {
            Player::A => (self.win, self.loss),
            Player::B => (self.loss, self.win),
        };

        WDLAbs {
            win_a,
            draw: self.draw,
            win_b,
        }
    }
}

impl<V> WDLAbs<V> {
    pub fn new(win_a: V, draw: V, win_b: V) -> Self {
        Self { win_a, draw, win_b }
    }
}

impl<V> WDL<V> {
    pub fn new(win: V, draw: V, loss: V) -> Self {
        WDL { win, draw, loss }
    }

    pub fn to_slice(self) -> [V; 3] {
        [self.win, self.draw, self.loss]
    }
}

impl<V: num_traits::Float> WDL<V> {
    pub fn nan() -> WDL<V> {
        WDL {
            win: V::nan(),
            draw: V::nan(),
            loss: V::nan(),
        }
    }

    pub fn normalized(self) -> WDL<V> {
        self / self.sum()
    }
}

impl<V: num_traits::Float> WDLAbs<V> {
    pub fn nan() -> WDLAbs<V> {
        WDLAbs {
            win_a: V::nan(),
            draw: V::nan(),
            win_b: V::nan(),
        }
    }
}

impl<V: num_traits::One + Default + PartialEq> WDLAbs<V> {
    pub fn try_to_outcome(self) -> Option<Outcome> {
        let outcomes = [Outcome::WonBy(Player::A), Outcome::Draw, Outcome::WonBy(Player::B)];
        outcomes.iter().copied().find(|&o| o.to_wdl_abs() == self)
    }
}

impl<V: num_traits::One + Default + PartialEq> WDL<V> {
    pub fn try_to_outcome_wdl(self) -> Option<OutcomeWDL> {
        let outcomes = [OutcomeWDL::Win, OutcomeWDL::Draw, OutcomeWDL::Loss];
        outcomes.iter().copied().find(|&o| o.to_wdl() == self)
    }
}

impl<V: Copy> WDL<V> {
    pub fn cast<W>(self) -> WDL<W>
    where
        V: Cast<W>,
    {
        WDL {
            win: self.win.cast(),
            draw: self.draw.cast(),
            loss: self.loss.cast(),
        }
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> WDL<V> {
    pub fn value(self) -> V {
        self.win - self.loss
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> WDLAbs<V> {
    pub fn value(self) -> ScalarAbs<V> {
        ScalarAbs::new(self.win_a - self.win_b)
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> WDL<V> {
    pub fn sum(self) -> V {
        self.win + self.draw + self.loss
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> WDLAbs<V> {
    pub fn sum(self) -> V {
        self.win_a + self.draw + self.win_b
    }
}

impl NonPov for Outcome {
    type Output = OutcomeWDL;
    fn pov(self, pov: Player) -> OutcomeWDL {
        match self {
            Outcome::WonBy(player) => {
                if player == pov {
                    OutcomeWDL::Win
                } else {
                    OutcomeWDL::Loss
                }
            }
            Outcome::Draw => OutcomeWDL::Draw,
        }
    }
}

impl Pov for OutcomeWDL {
    type Output = Outcome;
    fn un_pov(self, pov: Player) -> Outcome {
        match self {
            OutcomeWDL::Win => Outcome::WonBy(pov),
            OutcomeWDL::Draw => Outcome::Draw,
            OutcomeWDL::Loss => Outcome::WonBy(pov.other()),
        }
    }
}

impl<V: std::ops::Add<V, Output = V>> std::ops::Add<WDL<V>> for WDL<V> {
    type Output = WDL<V>;

    fn add(self, rhs: WDL<V>) -> Self::Output {
        WDL {
            win: self.win + rhs.win,
            draw: self.draw + rhs.draw,
            loss: self.loss + rhs.loss,
        }
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> std::ops::Sub<WDL<V>> for WDL<V> {
    type Output = WDL<V>;

    fn sub(self, rhs: WDL<V>) -> Self::Output {
        WDL {
            win: self.win - rhs.win,
            draw: self.draw - rhs.draw,
            loss: self.loss - rhs.loss,
        }
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> std::ops::AddAssign<WDL<V>> for WDL<V> {
    fn add_assign(&mut self, rhs: WDL<V>) {
        *self = *self + rhs;
    }
}

impl<V: Copy + std::ops::Mul<V, Output = V>> std::ops::Mul<V> for WDL<V> {
    type Output = WDL<V>;

    fn mul(self, rhs: V) -> Self::Output {
        WDL {
            win: self.win * rhs,
            draw: self.draw * rhs,
            loss: self.loss * rhs,
        }
    }
}

impl<V: Copy + std::ops::Div<V, Output = V>> std::ops::Div<V> for WDL<V> {
    type Output = WDL<V>;

    fn div(self, rhs: V) -> Self::Output {
        WDL {
            win: self.win / rhs,
            draw: self.draw / rhs,
            loss: self.loss / rhs,
        }
    }
}

impl<V: Default + Copy + std::ops::Add<Output = V>> std::iter::Sum<Self> for WDL<V> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, v| a + v)
    }
}

impl<'a, V: Default + Copy + std::ops::Add<Output = V>> std::iter::Sum<&'a Self> for WDL<V> {
    fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, &v| a + v)
    }
}

impl<V: std::ops::Add<V, Output = V>> std::ops::Add<WDLAbs<V>> for WDLAbs<V> {
    type Output = WDLAbs<V>;

    fn add(self, rhs: WDLAbs<V>) -> Self::Output {
        WDLAbs {
            win_a: self.win_a + rhs.win_a,
            draw: self.draw + rhs.draw,
            win_b: self.win_b + rhs.win_b,
        }
    }
}

impl<V: Copy + std::ops::Sub<V, Output = V>> std::ops::Sub<WDLAbs<V>> for WDLAbs<V> {
    type Output = WDLAbs<V>;

    fn sub(self, rhs: WDLAbs<V>) -> Self::Output {
        WDLAbs {
            win_a: self.win_a - rhs.win_a,
            draw: self.draw - rhs.draw,
            win_b: self.win_b - rhs.win_b,
        }
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> std::ops::AddAssign<WDLAbs<V>> for WDLAbs<V> {
    fn add_assign(&mut self, rhs: WDLAbs<V>) {
        *self = *self + rhs;
    }
}

impl<V: Copy + std::ops::Mul<V, Output = V>> std::ops::Mul<V> for WDLAbs<V> {
    type Output = WDLAbs<V>;

    fn mul(self, rhs: V) -> Self::Output {
        WDLAbs {
            win_a: self.win_a * rhs,
            draw: self.draw * rhs,
            win_b: self.win_b * rhs,
        }
    }
}

impl<V: Copy + std::ops::Div<V, Output = V>> std::ops::Div<V> for WDLAbs<V> {
    type Output = WDLAbs<V>;

    fn div(self, rhs: V) -> Self::Output {
        WDLAbs {
            win_a: self.win_a / rhs,
            draw: self.draw / rhs,
            win_b: self.win_b / rhs,
        }
    }
}

impl<V: Default + Copy + std::ops::Add<Output = V>> std::iter::Sum for WDLAbs<V> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, v| a + v)
    }
}
