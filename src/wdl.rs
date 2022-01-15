use cast_trait::Cast;
use internal_iterator::IntoInternalIterator;

use crate::board::{Outcome, Player};
use crate::util::internal_ext::{Control, InternalIteratorExt};

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

/// Trait to convert an absolute outcome to a relative one.
pub trait POV {
    type Output;

    /// View this outcome from the POV of `pov`.
    fn pov(self, pov: Player) -> Self::Output;
}

pub trait Flip {
    /// Flip this outcome.
    fn flip(self) -> Self;
}

impl OutcomeWDL {
    /// Convert this to a WDL with a one at the correct place and zero otherwise.
    pub fn to_wdl<V: num::One + num::Zero>(self) -> WDL<V> {
        match self {
            OutcomeWDL::Win => WDL {
                win: V::one(),
                draw: V::zero(),
                loss: V::zero(),
            },
            OutcomeWDL::Draw => WDL {
                win: V::zero(),
                draw: V::one(),
                loss: V::zero(),
            },
            OutcomeWDL::Loss => WDL {
                win: V::zero(),
                draw: V::zero(),
                loss: V::one(),
            },
        }
    }

    /// Convert a win to `1`, draw to `0` and loss to `-1`.
    pub fn sign<V: num::Zero + num::One + std::ops::Neg<Output = V>>(self) -> V {
        match self {
            OutcomeWDL::Win => V::one(),
            OutcomeWDL::Draw => V::zero(),
            OutcomeWDL::Loss => -V::one(),
        }
    }

    /// Convert a win to `inf`, draw to `0` and loss to `-inf`.
    pub fn inf_sign<V: num::Float>(self) -> V {
        match self {
            OutcomeWDL::Win => V::infinity(),
            OutcomeWDL::Draw => V::zero(),
            OutcomeWDL::Loss => V::neg_infinity(),
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

    pub fn best<I>(children: I) -> Option<OutcomeWDL>
    where
        I: IntoInternalIterator<Item = Option<OutcomeWDL>>,
    {
        let mut any_unknown = false;
        let mut all_known_are_loss = true;

        let control = children.into_internal_iter().for_each_control(|child| {
            match child {
                None => {
                    any_unknown = true;
                }
                Some(OutcomeWDL::Win) => {
                    return Control::Break(());
                }
                Some(OutcomeWDL::Draw) => {
                    all_known_are_loss = false;
                }
                Some(OutcomeWDL::Loss) => {}
            }

            Control::Continue
        });

        if control.is_some() {
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

impl<V: num::Float> WDL<V> {
    pub fn nan() -> WDL<V> {
        WDL {
            win: V::nan(),
            draw: V::nan(),
            loss: V::nan(),
        }
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

impl<V: num::One + num::Zero + PartialEq> WDL<V> {
    pub fn try_to_outcome_wdl(self) -> Result<OutcomeWDL, ()> {
        let outcomes = [OutcomeWDL::Win, OutcomeWDL::Draw, OutcomeWDL::Loss];
        outcomes.iter().copied().find(|&o| o.to_wdl() == self).ok_or(())
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

impl<V: Copy + std::ops::Add<V, Output = V>> WDL<V> {
    pub fn sum(self) -> V {
        self.win + self.draw + self.loss
    }
}

impl<I: POV> POV for Option<I> {
    type Output = Option<I::Output>;
    fn pov(self, pov: Player) -> Option<I::Output> {
        self.map(|inner| inner.pov(pov))
    }
}

impl<I: Flip> Flip for Option<I> {
    fn flip(self) -> Self {
        self.map(|inner| inner.flip())
    }
}

impl POV for Outcome {
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

impl Flip for OutcomeWDL {
    fn flip(self) -> Self {
        match self {
            OutcomeWDL::Win => OutcomeWDL::Loss,
            OutcomeWDL::Draw => OutcomeWDL::Draw,
            OutcomeWDL::Loss => OutcomeWDL::Win,
        }
    }
}

impl<V: Copy> Flip for WDL<V> {
    fn flip(self) -> Self {
        WDL {
            win: self.loss,
            draw: self.draw,
            loss: self.win,
        }
    }
}

impl<V: Copy + std::ops::Add<V, Output = V>> std::ops::Add<WDL<V>> for WDL<V> {
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

impl<V: Default + Copy + std::ops::Add<Output = V>> std::iter::Sum for WDL<V> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, v| a + v)
    }
}
