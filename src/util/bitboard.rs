use std::fmt::{Display, Formatter};

use crate::util::bits::{BitIter, get_nth_set_bit};
use crate::util::coord::Coord8;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BitBoard8(u64);

impl BitBoard8 {
    pub const EMPTY: BitBoard8 = BitBoard8(0);
    pub const FULL: BitBoard8 = BitBoard8(!0);

    pub const FULL_FOR_SIZE: [BitBoard8; 9] = [
        BitBoard8(0x0000000000000000),
        BitBoard8(0x0000000000000001),
        BitBoard8(0x0000000000000303),
        BitBoard8(0x0000000000070707),
        BitBoard8(0x000000000f0f0f0f),
        BitBoard8(0x0000001f1f1f1f1f),
        BitBoard8(0x00003f3f3f3f3f3f),
        BitBoard8(0x007f7f7f7f7f7f7f),
        BitBoard8(0xffffffffffffffff),
    ];

    #[must_use]
    pub const fn new(bits: u64) -> BitBoard8 {
        BitBoard8(bits)
    }

    #[must_use]
    pub fn coord(coord: Coord8) -> BitBoard8 {
        BitBoard8(1 << coord.index())
    }

    #[must_use]
    pub fn has(self, coord: Coord8) -> bool {
        (self.0 >> coord.index()) & 1 != 0
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[must_use]
    pub fn get_nth(self, index: u32) -> Coord8 {
        Coord8::from_index(get_nth_set_bit(self.0, index))
    }

    #[must_use]
    pub fn set(self, coord: Coord8) -> Self {
        BitBoard8(self.0 | (1 << coord.index()))
    }

    #[must_use]
    pub fn clear(self, coord: Coord8) -> Self {
        BitBoard8(self.0 & !(1 << coord.index()))
    }

    pub fn left(self) -> Self {
        BitBoard8((self.0 >> 1) & 0x7f7f7f7f7f7f7f7f)
    }

    pub fn right(self) -> Self {
        BitBoard8((self.0 << 1) & 0xfefefefefefefefe)
    }

    pub fn down(self) -> Self {
        BitBoard8((self.0 >> 8) & 0x00ffffffffffffff)
    }

    pub fn up(self) -> Self {
        BitBoard8((self.0 << 8) & 0xffffffffffffff00)
    }

    pub fn orthogonal(self) -> Self {
        self.left() | self.right() | self.up() | self.down()
    }

    pub fn diagonal(self) -> Self {
        self.left().up() | self.right().up() | self.left().down() | self.right().down()
    }

    pub fn adjacent(self) -> Self {
        self.orthogonal() | self.diagonal()
    }

    pub fn ring(self) -> Self {
        // this cannot be simplified to `self.adjacent().adjacent() & ~self`,
        //   that only works for a single or a few non-overlapping bits
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

    pub fn flip_x(self) -> BitBoard8 {
        // reverse_bits is a transpose, swap_bytes a vertical flip
        BitBoard8(self.0.reverse_bits().swap_bytes())
    }

    pub fn flip_y(self) -> BitBoard8 {
        BitBoard8(self.0.swap_bytes())
    }
}

impl IntoIterator for BitBoard8 {
    type Item = Coord8;
    type IntoIter = std::iter::Map<BitIter<u64>, fn(u8) -> Coord8>;

    fn into_iter(self) -> Self::IntoIter {
        BitIter::new(self.0).map(|i| Coord8::from_index(i))
    }
}

impl Display for BitBoard8 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for y in (0..8).rev() {
            for x in 0..8 {
                let coord = Coord8::from_xy(x, y);
                write!(f, "{}", if self.has(coord) { '1' } else { '.' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

mod operations {
    use super::*;

    impl std::ops::BitOr for BitBoard8 {
        type Output = BitBoard8;

        fn bitor(self, rhs: Self) -> Self::Output {
            BitBoard8(self.0 | rhs.0)
        }
    }

    impl std::ops::BitAnd for BitBoard8 {
        type Output = BitBoard8;

        fn bitand(self, rhs: Self) -> Self::Output {
            BitBoard8(self.0 & rhs.0)
        }
    }

    impl std::ops::BitXor for BitBoard8 {
        type Output = BitBoard8;

        fn bitxor(self, rhs: Self) -> Self::Output {
            BitBoard8(self.0 ^ rhs.0)
        }
    }

    impl std::ops::Not for BitBoard8 {
        type Output = BitBoard8;

        fn not(self) -> Self::Output {
            BitBoard8(!self.0)
        }
    }

    impl std::ops::BitOrAssign for BitBoard8 {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0
        }
    }

    impl std::ops::BitAndAssign for BitBoard8 {
        fn bitand_assign(&mut self, rhs: Self) {
            self.0 &= rhs.0
        }
    }

    impl std::ops::BitXorAssign for BitBoard8 {
        fn bitxor_assign(&mut self, rhs: Self) {
            self.0 ^= rhs.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_jump() {
        let cases = [
            (Coord8::from_xy(7, 7), 0x40c0000000000000, 0x2020e00000000000),
            (Coord8::from_xy(0, 7), 0x0203000000000000, 0x0404070000000000),
            (Coord8::from_xy(7, 0), 0x000000000000c040, 0x0000000000e02020),
            (Coord8::from_xy(0, 0), 0x0000000000000302, 0x0000000000070404),
        ];

        for (coord, copy, jump) in cases {
            println!("Coord {:?}", coord);

            let actual_copy = BitBoard8::coord(coord).adjacent();
            println!("{}", actual_copy);

            let actual_jump = BitBoard8::coord(coord).ring();
            println!("{}", actual_jump);

            assert_eq!(actual_copy, BitBoard8(copy), "Wrong copy targets");
            assert_eq!(actual_jump, BitBoard8(jump), "Wrong jump targets");
        }
    }

    #[test]
    fn flip() {
        let board = BitBoard8::new(0x16101010000606);
        let expected_flip_x = BitBoard8::new(0x68080808006060);
        let expected_flip_y = BitBoard8::new(0x606001010101600);

        println!("{}", board);
        println!("{}", board.flip_x());
        println!("{}", board.flip_y());

        assert_eq!(board.flip_x(), expected_flip_x);
        assert_eq!(board.flip_y(), expected_flip_y);
    }
}
