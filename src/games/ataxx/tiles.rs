use std::fmt::{Display, Formatter};

use crate::games::ataxx::mv::Coord;
use crate::games::ataxx::AtaxxBoard;
use crate::symmetry::D4Symmetry;
use crate::util::bits::{get_nth_set_bit, BitIter};

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
    pub fn full(size: u8) -> Tiles {
        const FULL_MASKS: [u64; 9] = [
            0x0000000000000000,
            0x0000000000000001,
            0x0000000000000303,
            0x0000000000070707,
            0x000000000f0f0f0f,
            0x0000001f1f1f1f1f,
            0x00003f3f3f3f3f3f,
            0x007f7f7f7f7f7f7f,
            0xffffffffffffffff,
        ];

        assert!(size <= 8);
        Tiles(FULL_MASKS[size as usize])
    }

    pub fn empty() -> Tiles {
        Tiles(0)
    }

    pub fn coord(coord: Coord) -> Tiles {
        Tiles(1 << coord.sparse_i())
    }

    pub fn inner(self) -> u64 {
        self.0
    }

    pub fn has(self, coord: Coord) -> bool {
        (self.0 >> coord.sparse_i()) & 1 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
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

    pub fn not(self, size: u8) -> Self {
        Tiles(!self.0) & Tiles::full(size)
    }

    pub fn left(self, size: u8) -> Self {
        Tiles(self.0 >> 1) & Tiles::full(size)
    }

    pub fn right(self, size: u8) -> Self {
        Tiles(self.0 << 1) & Tiles::full(size)
    }

    pub fn down(self, size: u8) -> Self {
        Tiles(self.0 >> 8) & Tiles::full(size)
    }

    pub fn up(self, size: u8) -> Self {
        Tiles(self.0 << 8) & Tiles::full(size)
    }

    pub fn copy_targets(self, size: u8) -> Self {
        // counterclockwise starting from left
        self.left(size)
            | self.left(size).down(size)
            | self.down(size)
            | self.right(size).down(size)
            | self.right(size)
            | self.right(size).up(size)
            | self.up(size)
            | self.left(size).up(size)
    }

    pub fn jump_targets(self, size: u8) -> Self {
        // counterclockwise starting from left.left
        self.left(size).left(size)
            | self.left(size).left(size).down(size)
            | self.left(size).left(size).down(size).down(size)
            | self.left(size).down(size).down(size)
            | self.down(size).down(size)
            | self.right(size).down(size).down(size)
            | self.right(size).right(size).down(size).down(size)
            | self.right(size).right(size).down(size)
            | self.right(size).right(size)
            | self.right(size).right(size).up(size)
            | self.right(size).right(size).up(size).up(size)
            | self.right(size).up(size).up(size)
            | self.up(size).up(size)
            | self.left(size).up(size).up(size)
            | self.left(size).left(size).up(size).up(size)
            | self.left(size).left(size).up(size)
    }

    pub fn map(self, size: u8, sym: D4Symmetry) -> Tiles {
        assert!(size < AtaxxBoard::MAX_SIZE);

        let mut result = Tiles::empty();
        for c in self {
            result = result.set(c.map(size, sym))
        }
        result
    }
}

impl Display for Tiles {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for y in (0..8).rev() {
            for x in 0..8 {
                let coord = Coord::from_xy(x, y);
                write!(f, "{}", if self.has(coord) { '1' } else { '.' })?;
            }
            writeln!(f)?;
        }
        Ok(())
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
