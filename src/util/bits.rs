//! Utilities with compact bit data structures.

use std::num::Wrapping;
use std::ops::Neg;

use num_traits::{PrimInt, Unsigned, WrappingSub, Zero};

#[derive(Debug)]
/// Iterator over the indices of the set bits of an integer,
/// from least to most significant.
///
/// # Example
///
/// ```
/// use board_game::util::bits::BitIter;
/// let b = BitIter::new(0b10011u32);
/// assert_eq!(b.collect::<Vec<_>>(), vec![0, 1, 4]);
/// ```
pub struct BitIter<N: PrimInt + Unsigned> {
    left: N,
}

impl<N: PrimInt + Unsigned> BitIter<N> {
    pub fn new(left: N) -> Self {
        BitIter { left }
    }
}

impl<N: PrimInt + Unsigned> Iterator for BitIter<N> {
    type Item = u8;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        //TODO report bug to intel-rust that self.left.is_zero() complains about a missing trait
        if self.left == N::zero() {
            None
        } else {
            let index = self.left.trailing_zeros() as u8;
            self.left = self.left & (self.left - N::one());
            Some(index)
        }
    }
}

pub fn get_nth_set_bit<N: PrimInt + Unsigned + WrappingSub>(mut x: N, n: u32) -> u8 {
    for _ in 0..n {
        x = x & x.wrapping_sub(&N::one());
    }
    debug_assert!(x != N::zero());
    x.trailing_zeros() as u8
}

/// Iterator over all subsets of the given mask.
///
/// If the mask has `N` set bits this yields `2 ** N` values.
///
/// Implementation based on https://analog-hors.github.io/writing/magic-bitboards/
/// and https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#All_Subsets_of_any_Set
#[derive(Debug)]
pub struct SubSetIterator {
    start: bool,
    curr: u64,
    mask: u64,
}

impl SubSetIterator {
    pub fn new(mask: u64) -> Self {
        Self {
            start: true,
            curr: 0,
            mask,
        }
    }
}

impl Iterator for SubSetIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == 0 && !self.start {
            return None;
        }
        self.start = false;

        let result = self.curr;
        self.curr = (self.curr.wrapping_sub(self.mask)) & self.mask;
        Some(result)
    }
}

/// Iterator over all subsets of the given mask that have `M` bits set.
///
/// If the mask has `N` set bits this yields `nCr(N, M)` values.
/// Only yields any values if `N >= M`.
///
/// Implementation based on https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#Snoobing_any_Sets
#[derive(Debug)]
pub struct SubSetCountIterator {
    mask: u64,
    curr: u64,
    m: u32,
}

impl SubSetCountIterator {
    pub fn new(mask: u64, m: u32) -> Self {
        if m > mask.count_ones() {
            // don't yield any values
            return SubSetCountIterator {
                mask,
                curr: 0,
                m: u32::MAX,
            };
        }

        if m == 0 {
            // yield zero once
            return SubSetCountIterator { mask, curr: 0, m: 0 };
        }

        let start = {
            // TODO is there a cleaner/faster way to write this?
            let mut left = m;
            let mut start = 0;

            for i in 0..64 {
                if left == 0 {
                    break;
                }

                if mask & (1 << i) != 0 {
                    left -= 1;
                    start |= 1 << i;
                }
            }

            start
        };

        SubSetCountIterator { mask, curr: start, m }
    }
}

impl Iterator for SubSetCountIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO find a better way to do this
        if self.m == 0 {
            self.m = u32::MAX;
            return Some(0);
        }
        if self.m == u32::MAX {
            return None;
        }
        if self.curr.count_ones() != self.m {
            self.m = u32::MAX;
            return None;
        }

        if self.curr == 0 {
            None
        } else {
            let result = self.curr;
            self.curr = snoob_masked(self.curr, self.mask);
            Some(result)
        }
    }
}

/// Based on https://www.chessprogramming.org/Traversing_Subsets_of_a_Set#Snoobing_any_Sets
fn snoob_masked(sub: u64, set: u64) -> u64 {
    let mut sub = Wrapping(sub);
    let mut set = Wrapping(set);

    let mut tmp = sub - Wrapping(1);
    let mut rip = set & (tmp + (sub & sub.neg()) - set);

    sub = (tmp & sub) ^ rip;
    sub &= sub - Wrapping(1);

    while !sub.is_zero() {
        tmp = set & set.neg();

        rip ^= tmp;
        set ^= tmp;

        sub &= sub - Wrapping(1);
    }

    rip.0
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::util::bits::{snoob_masked, SubSetCountIterator, SubSetIterator};

    #[test]
    fn subset_iterator_empty() {
        assert_eq!(1, SubSetIterator::new(0).count());
    }

    #[test]
    fn subset_iterator_standard() {
        let mask = 0b01001100;

        // check that the output values are right
        let iter = SubSetIterator::new(mask);
        let values = iter.collect_vec();
        let expected = vec![
            0b00000000, 0b00000100, 0b00001000, 0b00001100, 0b01000000, 0b01000100, 0b01001000, 0b01001100,
        ];
        assert_eq!(values, expected);

        // check that it only iterates once
        let mut iter = SubSetIterator::new(mask);
        assert_eq!(8, iter.by_ref().count());
        assert_eq!(0, iter.count());
    }

    #[test]
    fn subset_count_iterator_standard() {
        for mask in [0b01001100, 0b11111111] {
            println!("mask={:b}", mask);

            for m in 0..8 {
                println!("m={}", m);

                let expected = SubSetIterator::new(mask).filter(|v| v.count_ones() == m).collect_vec();
                let actual = SubSetCountIterator::new(mask, m).collect_vec();

                for a in &actual {
                    println!("  {:b}", a);
                }

                assert_eq!(expected, actual);
            }
        }
    }

    #[test]
    fn snoob_random() {
        fn test(x: u64, y: u64, z: u64) {
            let a = snoob_masked(x, y);
            assert_eq!(a, z, "Expected 0x{:x}, got 0x{:x}", z, a);
        }

        // test values generated by running C version of the function on random inputs
        test(0x7283e4c96896188c, 0x706b7f2de031bf37, 0x866246011a627);
        test(0xfad96ea1180d0e12, 0x76509766802e6373, 0x7250004400204153);
        test(0xe5a6f8869eb40f35, 0xf16528ff4aace975, 0xf06420780aa8c835);
        test(0x9690d5c2f7a35fe0, 0x74d4cb118f57c2a5, 0x5440c100871442a5);
        test(0x65282b0a08ebb2ec, 0xd1a0372c789578cd, 0x90200420688140cd);
        test(0x3dac1b1d3add1987, 0xf9f42a39293c9190, 0xb8400a19281c1000);
        test(0xa4ff0d56382d1243, 0xdfadd0cd921aba0, 0x8facd0480208900);
        test(0xebec27ac83bd6464, 0x39a0a563e5a54560, 0x9a0252361a54060);
        test(0x3b6a9e51e98ee1dc, 0x4c06832895fdc792, 0x28000846cc592);
        test(0xc9afc50b67c6c180, 0xb96b34721235db30, 0xa92b246202251930);
        test(0x950759b98d0b9d44, 0x2fb8f78ae913838f, 0x25b0958048038207);
        test(0x41014bfee54406c9, 0xdd559e0e54ea7ed9, 0x41018c0644a27809);
        test(0x63ed18bf633dbce0, 0xe68adf1c4dbf1fd6, 0x6402161c488102d6);
        test(0xf8a1baa1355ebec3, 0x90765cca6bdae99, 0x52108201cae91);
        test(0xd157a686cccad263, 0x6f7d04460b4d0672, 0x4e2504060a000402);
        test(0xd93c30843c3fdb27, 0x3a5c127454f3448, 0x2218023010f1000);
    }
}
