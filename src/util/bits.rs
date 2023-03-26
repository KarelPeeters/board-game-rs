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

    let tmp = sub - Wrapping(1);
    let mut rip = set & (tmp + (sub & sub.neg()) - set);

    sub = (tmp & sub) ^ rip;
    sub &= sub - Wrapping(1);

    while !sub.is_zero() {
        let tmp = set & set.neg();
        set ^= tmp;

        rip ^= tmp;
        set ^= tmp;

        sub &= sub - Wrapping(1);
    }

    rip.0
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::util::bits::{SubSetCountIterator, SubSetIterator};

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
        let mask = 0b01001100;
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
