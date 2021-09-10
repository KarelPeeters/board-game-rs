use std::fmt::{Display, Formatter};

use crate::games::ataxx::mv::Coord;
use crate::symmetry::D4Symmetry;
use crate::util::bits::{BitIter, get_nth_set_bit};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Tiles(u64);

impl IntoIterator for Tiles {
    type Item = Coord;
    type IntoIter = std::iter::Map<BitIter<u64>, fn(u8) -> Coord>;

    fn into_iter(self) -> Self::IntoIter {
        BitIter::new(self.0).map(|i| Coord::from_sparse_i(i as u8))
    }
}

impl Tiles {
    pub const FULL_MASK: u64 = 0x7F_7F_7F_7F_7F_7F_7F;
    pub const CORNERS_A: Tiles = Tiles(0x_01_00_00_00_00_00_40);
    pub const CORNERS_B: Tiles = Tiles(0x_40_00_00_00_00_00_01);

    pub fn full() -> Tiles {
        Tiles(Self::FULL_MASK)
    }

    pub fn empty() -> Tiles {
        Tiles(0)
    }

    pub fn coord(coord: Coord) -> Tiles {
        Tiles(1 << coord.sparse_i())
    }

    pub fn has(self, coord: Coord) -> bool {
        (self.0 >> coord.sparse_i()) & 1 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn is_full(self) -> bool {
        self.0 == Self::FULL_MASK
    }

    pub fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    pub fn get_nth(self, index: u32) -> Coord {
        Coord::from_sparse_i(get_nth_set_bit(self.0, index))
    }

    #[must_use]
    pub fn set(self, coord: Coord) -> Self {
        Tiles(self.0 | (1 << coord.sparse_i()))
    }

    #[must_use]
    pub fn clear(self, coord: Coord) -> Self {
        Tiles(self.0 & !(1 << coord.sparse_i()))
    }

    pub fn left(self) -> Self {
        Tiles((self.0 >> 1) & Self::FULL_MASK)
    }

    pub fn right(self) -> Self {
        Tiles((self.0 << 1) & Self::FULL_MASK)
    }

    pub fn down(self) -> Self {
        Tiles((self.0 >> 8) & Self::FULL_MASK)
    }

    pub fn up(self) -> Self {
        Tiles((self.0 << 8) & Self::FULL_MASK)
    }

    pub fn copy_targets(self) -> Self {
        // counterclockwise starting from left
        self.left() | self.left().down() | self.down() | self.right().down()
            | self.right() | self.right().up() | self.up() | self.left().up()
    }

    pub fn jump_targets(self) -> Self {
        // counterclockwise starting from left.left
        self.left().left()
            | self.left().left().down()
            | self.left().left().down().down()
            | self.left().down().down()
            | self.down().down()
            | self.right().down().down()
            | self.right().right().down().down()
            | self.right().right().down()
            | self.right().right()
            | self.right().right().up()
            | self.right().right().up().up()
            | self.right().up().up()
            | self.up().up()
            | self.left().up().up()
            | self.left().left().up().up()
            | self.left().left().up()
    }

    pub fn map(self, sym: D4Symmetry) -> Tiles {
        let mut result = Tiles::empty();
        for c in self {
            result = result.set(c.map(sym))
        }
        result
    }
}

impl Display for Tiles {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert_eq!(self.0 & Tiles::FULL_MASK, self.0);
        for y in (0..7).rev() {
            for x in 0..7 {
                let coord = Coord::from_xy(x, y);
                write!(f, "{}", if self.has(coord) { '1' } else { '.' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl std::ops::Not for Tiles {
    type Output = Tiles;

    fn not(self) -> Self::Output {
        Tiles((!self.0) & Tiles::FULL_MASK)
    }
}

impl std::ops::BitOr for Tiles {
    type Output = Tiles;

    fn bitor(self, rhs: Self) -> Self::Output {
        Tiles(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Tiles {
    type Output = Tiles;

    fn bitand(self, rhs: Self) -> Self::Output {
        Tiles(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for Tiles {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl std::ops::BitAndAssign for Tiles {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}
