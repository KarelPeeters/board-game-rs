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
        BitIter::new(self.0).map(|i| Coord(i as u8))
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
        Tiles(1 << coord.0)
    }

    pub fn has(self, coord: Coord) -> bool {
        (self.0 >> coord.0) & 1 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    pub fn get_nth(self, index: u32) -> Coord {
        Coord(get_nth_set_bit(self.0, index))
    }

    #[must_use]
    pub fn set(self, coord: Coord) -> Self {
        Tiles(self.0 | (1 << coord.0))
    }

    #[must_use]
    pub fn clear(self, coord: Coord) -> Self {
        Tiles(self.0 & !(1 << coord.0))
    }

    pub fn not(self, size: u8) -> Self {
        Tiles(!self.0) & Tiles::full(size)
    }

    fn left(self) -> Self {
        Tiles((self.0 >> 1) & 0x7f7f7f7f7f7f7f7f)
    }

    fn right(self) -> Self {
        Tiles((self.0 << 1) & 0xfefefefefefefefe)
    }

    fn down(self) -> Self {
        Tiles((self.0 >> 8) & 0x00ffffffffffffff)
    }

    fn up(self) -> Self {
        Tiles((self.0 << 8) & 0xffffffffffffff00)
    }

    pub fn copy_targets(self, size: u8) -> Self {
        // counterclockwise starting from left
        let result = self.left()
            | self.left().down()
            | self.down()
            | self.right().down()
            | self.right()
            | self.right().up()
            | self.up()
            | self.left().up();
        result & Tiles::full(size)
    }

    pub fn jump_targets(self, size: u8) -> Self {
        // counterclockwise starting from left.left
        let result = self.left().left()
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
            | self.left().left().up();

        result & Tiles::full(size)
    }

    pub fn map(self, size: u8, sym: D4Symmetry) -> Tiles {
        assert!(size <= AtaxxBoard::MAX_SIZE);

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

#[cfg(test)]
mod tests {
    use crate::games::ataxx::Coord;

    use super::Tiles;

    #[test]
    fn copy_jump() {
        let cases = [
            (8, Coord::from_xy(7, 7), 0x40c0000000000000, 0x2020e00000000000),
            (8, Coord::from_xy(0, 7), 0x0203000000000000, 0x0404070000000000),
            (8, Coord::from_xy(7, 0), 0x000000000000c040, 0x0000000000e02020),
            (8, Coord::from_xy(0, 0), 0x0000000000000302, 0x0000000000070404),
        ];

        for (size, coord, copy, jump) in cases {
            println!("Size {}, coord {:?}", size, coord);

            let actual_copy = Tiles::coord(coord).copy_targets(size);
            println!("{}", actual_copy);

            let actual_jump = Tiles::coord(coord).jump_targets(size);
            println!("{}", actual_jump);

            assert_eq!(actual_copy, Tiles(copy), "Wrong copy targets");
            assert_eq!(actual_jump, Tiles(jump), "Wrong jump targets");
        }
    }
}
