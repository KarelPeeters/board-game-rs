use std::fmt::{Debug, Display, Formatter};

use crate::util::bits::{get_nth_set_bit, BitIter};
use crate::util::coord::Coord8;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct BitBoard8(pub u64);

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
    pub const fn coord(coord: Coord8) -> BitBoard8 {
        BitBoard8(1 << coord.index())
    }

    #[must_use]
    pub const fn coord_option(coord: Option<Coord8>) -> BitBoard8 {
        match coord {
            Some(coord) => BitBoard8::coord(coord),
            None => BitBoard8::EMPTY,
        }
    }

    #[must_use]
    pub fn from_coords(coords: impl IntoIterator<Item = Coord8>) -> BitBoard8 {
        coords.into_iter().fold(BitBoard8::EMPTY, |b, c| b.set(c))
    }

    #[must_use]
    pub const fn has(self, coord: Coord8) -> bool {
        (self.0 >> coord.index()) & 1 != 0
    }

    #[must_use]
    pub const fn none(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    #[must_use]
    pub const fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[must_use]
    pub fn get_nth(self, index: u32) -> Coord8 {
        Coord8::from_index(get_nth_set_bit(self.0, index))
    }

    #[must_use]
    pub const fn set(self, coord: Coord8) -> Self {
        BitBoard8(self.0 | (1 << coord.index()))
    }

    #[must_use]
    pub const fn clear(self, coord: Coord8) -> Self {
        BitBoard8(self.0 & !(1 << coord.index()))
    }

    #[must_use]
    pub const fn left(self) -> Self {
        BitBoard8((self.0 >> 1) & 0x7f7f7f7f7f7f7f7f)
    }

    #[must_use]
    pub const fn right(self) -> Self {
        BitBoard8((self.0 << 1) & 0xfefefefefefefefe)
    }

    #[must_use]
    pub const fn down(self) -> Self {
        BitBoard8((self.0 >> 8) & 0x00ffffffffffffff)
    }

    #[must_use]
    pub const fn up(self) -> Self {
        BitBoard8((self.0 << 8) & 0xffffffffffffff00)
    }

    #[must_use]
    pub const fn orthogonal(self) -> Self {
        let x = self.0;
        let y = (x >> 1) & 0x7f7f7f7f7f7f7f7f | (x << 1) & 0xfefefefefefefefe | x << 8 | x >> 8;
        BitBoard8(y)
    }

    #[must_use]
    pub const fn diagonal(self) -> Self {
        let x = self.0;
        let y = (x << 7 | x >> 9) & 0x7f7f7f7f7f7f7f7f | (x >> 7 | x << 9) & 0xfefefefefefefefe;
        BitBoard8(y)
    }

    #[must_use]
    pub const fn adjacent(self) -> Self {
        // writing this out yields the exact same shifts and masks as just doing this
        BitBoard8(self.orthogonal().0 | self.diagonal().0)
    }

    #[must_use]
    pub const fn ring(self) -> Self {
        let x = self.0;

        let left_2 = (x >> 2 | x >> 10 | x >> 18 | x << 6 | x << 14) & 0x3f3f3f3f3f3f3f3f;
        let left_1 = (x >> 17 | x << 15) & 0x7f7f7f7f7f7f7f7f;
        let center = x << 16 | x >> 16;
        let right_1 = (x << 17 | x >> 15) & 0xfefefefefefefefe;
        let right_2 = (x << 2 | x << 10 | x << 18 | x >> 6 | x >> 14) & 0xfcfcfcfcfcfcfcfc;

        let y = left_2 | left_1 | center | right_1 | right_2;
        BitBoard8(y)
    }

    /// The same as `(self.adjacent() | self.ring()) & !self` but expected to faster.
    #[must_use]
    pub const fn adjacent_or_ring_not_self(self) -> Self {
        let x = self.0;
        let line = (x << 2) & 0xfcfcfcfcfcfcfcfc
            | (x << 1) & 0xfefefefefefefefe
            | x
            | (x >> 1) & 0x7f7f7f7f7f7f7f7f
            | (x >> 2) & 0x3f3f3f3f3f3f3f3f;
        let y = (line | line << 8 | line >> 8 | line << 16 | line >> 16) & !x;
        debug_assert!(y == (self.adjacent().0 | self.ring().0) & !self.0);
        BitBoard8(y)
    }

    #[must_use]
    pub const fn flip_x(self) -> BitBoard8 {
        // reverse_bits flips both x and y, so undo y
        BitBoard8(self.0.reverse_bits().swap_bytes())
    }

    #[must_use]
    pub const fn flip_y(self) -> BitBoard8 {
        BitBoard8(self.0.swap_bytes())
    }

    #[must_use]
    pub const fn transpose(self) -> BitBoard8 {
        // implementation from Hacker's Delight, 2nd Edition
        let x = self.0;
        BitBoard8(
            x & 0x8040201008040201
                | (x & 0x0080402010080402) << 7
                | (x & 0x0000804020100804) << 14
                | (x & 0x0000008040201008) << 21
                | (x & 0x0000000080402010) << 28
                | (x & 0x0000000000804020) << 35
                | (x & 0x0000000000008040) << 42
                | (x & 0x0000000000000080) << 49
                | (x >> 7) & 0x0080402010080402
                | (x >> 14) & 0x0000804020100804
                | (x >> 21) & 0x0000008040201008
                | (x >> 28) & 0x0000000080402010
                | (x >> 35) & 0x0000000000804020
                | (x >> 42) & 0x0000000000008040
                | (x >> 49) & 0x0000000000000080,
        )
    }
}

impl IntoIterator for BitBoard8 {
    type Item = Coord8;
    type IntoIter = std::iter::Map<BitIter<u64>, fn(u8) -> Coord8>;

    fn into_iter(self) -> Self::IntoIter {
        BitIter::new(self.0).map(Coord8::from_index)
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

impl Debug for BitBoard8 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BitBoard8(")?;
        for y in (0..8).rev() {
            for x in 0..8 {
                let coord = Coord8::from_xy(x, y);
                write!(f, "{}", if self.has(coord) { '1' } else { '.' })?;
            }
            if y != 0 {
                write!(f, "/")?;
            }
        }
        write!(f, ")")?;
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
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};

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
        let board = BitBoard8(0x16101010000606);
        let expected_flip_x = BitBoard8(0x68080808006060);
        let expected_flip_y = BitBoard8(0x606001010101600);

        println!("{}", board);
        println!("{}", board.flip_x());
        println!("{}", board.flip_y());

        assert_eq!(board.flip_x(), expected_flip_x);
        assert_eq!(board.flip_y(), expected_flip_y);
    }

    fn ring_slow(board: BitBoard8) -> BitBoard8 {
        board.left().left()
            | board.left().left().down()
            | board.left().left().down().down()
            | board.left().down().down()
            | board.down().down()
            | board.right().down().down()
            | board.right().right().down().down()
            | board.right().right().down()
            | board.right().right()
            | board.right().right().up()
            | board.right().right().up().up()
            | board.right().up().up()
            | board.up().up()
            | board.left().up().up()
            | board.left().left().up().up()
            | board.left().left().up()
    }

    fn assert_board_eq(expected: BitBoard8, actual: BitBoard8) {
        if expected != actual {
            assert_eq!(expected.to_string(), actual.to_string());
            assert_eq!(expected, actual);
        } else {
            println!("ok");
        }
    }

    fn test_spatial_all(board: BitBoard8) {
        println!("board\n{}", board);

        print!("orthogonal: ");
        assert_board_eq(
            board.left() | board.right() | board.up() | board.down(),
            board.orthogonal(),
        );
        print!("diagonal: ");
        assert_board_eq(
            board.left().up() | board.right().up() | board.left().down() | board.right().down(),
            board.diagonal(),
        );
        print!("adjacent: ");
        assert_board_eq(board.orthogonal() | board.diagonal(), board.adjacent());
        print!("ring: ");
        assert_board_eq(ring_slow(board), board.ring());

        println!();
    }

    #[test]
    fn spatial() {
        for coord in Coord8::all() {
            println!("{:?} index {}", coord, coord.index());
            test_spatial_all(BitBoard8::coord(coord));
        }

        let mut rng = SmallRng::from_entropy();
        for _ in 0..128 {
            let board = BitBoard8(rng.gen());
            test_spatial_all(board);
        }
    }
}
