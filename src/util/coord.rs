use std::cmp::max;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Coord<const X: u8, const Y: u8> {
    index: u8,
}

pub type Coord3 = Coord<3, 3>;
pub type Coord8 = Coord<8, 8>;

pub type CoordAllIter<C> = std::iter::Map<std::ops::Range<u8>, fn(u8) -> C>;

impl<const X: u8, const Y: u8> Coord<X, Y> {
    pub fn from_index(index: u8) -> Self {
        assert!(index < X * Y);
        Coord { index }
    }

    pub fn from_xy(x: u8, y: u8) -> Self {
        assert!(x < X);
        assert!(y < Y);
        Coord { index: x + X * y }
    }

    pub fn all() -> CoordAllIter<Self> {
        (0..X * Y).map(|index| Coord::from_index(index))
    }

    pub fn index(self) -> u8 {
        self.index
    }

    pub fn x(self) -> u8 {
        self.index % X
    }

    pub fn y(self) -> u8 {
        self.index / X
    }

    pub fn manhattan_distance(self, other: Coord<X, Y>) -> u8 {
        let dx = self.x().abs_diff(other.x());
        let dy = self.y().abs_diff(other.y());
        dx + dy
    }

    pub fn diagonal_distance(self, other: Coord<X, Y>) -> u8 {
        let dx = self.x().abs_diff(other.x());
        let dy = self.y().abs_diff(other.y());
        max(dx, dy)
    }

    pub fn cast<const X2: u8, const Y2: u8>(self) -> Coord<X2, Y2> {
        Coord::<X2, Y2>::from_xy(self.x(), self.y())
    }
}

impl<const X: u8, const Y: u8> Display for Coord<X, Y> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x(), self.y())
    }
}
