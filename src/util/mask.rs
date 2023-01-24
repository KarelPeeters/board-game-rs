use crate::util::bitboard::BitBoard8;
use crate::util::coord::Coord8;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Copy, Clone)]
pub struct Mask {
    one: u64,
    zero: u64,
}

/// Find a set of masks that covers the given requirements.
/// This basically solves the [Set cover problem](https://en.wikipedia.org/wiki/Set_cover_problem).
pub fn cover_masks(requirements: &[Mask]) -> Vec<(Mask, Vec<usize>)> {
    // TODO this is a greedy implementation, write a better one
    //   doesn't really need to be fast, just optimal
    let mut result: Vec<(Mask, Vec<usize>)> = vec![];
    'outer: for (req_index, &req) in requirements.iter().enumerate() {
        for (cand, map) in &mut result {
            if let Some(new) = cand.merge(req) {
                *cand = new;
                map.push(req_index);
                continue 'outer;
            }
        }
        result.push((req, vec![req_index]));
    }
    result
}

impl Mask {
    fn new(one: u64, zero: u64) -> Option<Self> {
        if one & zero == 0 {
            Some(Mask { one, zero })
        } else {
            None
        }
    }

    fn merge(self, other: Mask) -> Option<Mask> {
        Mask::new(self.one | other.one, self.zero | other.zero)
    }

    pub fn one(&self) -> u64 {
        self.one
    }

    pub fn zero(&self) -> u64 {
        self.zero
    }
}

pub type Operation = fn(BitBoard8) -> BitBoard8;

pub fn find_requirements(ops: &[(i32, Operation)], result_mask: u64) -> Vec<Mask> {
    let mut requirements = vec![];

    for &(shift, op) in ops {
        let mut mask_one = 0;
        let mut mask_zero = 0;

        for coord in Coord8::all() {
            let before = BitBoard8::coord(coord).0 & result_mask;
            let after_correct = op(BitBoard8(before)).0 & result_mask;
            let after_shift = apply_shift(before, shift) & result_mask;

            assert_eq!(after_correct & !after_shift, 0, "Shift must cover correct");
            mask_one |= after_correct;
            mask_zero |= after_shift & !after_correct;
        }

        requirements.push(Mask::new(mask_one, mask_zero).unwrap());
    }

    requirements
}

impl Display for Mask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for y in (0..8).rev() {
            for x in 0..8 {
                let coord = Coord8::from_xy(x, y);

                let c = match (BitBoard8(self.one).has(coord), BitBoard8(self.zero).has(coord)) {
                    (true, false) => '1',
                    (false, true) => '0',
                    (false, false) => '.',
                    (true, true) => 'x',
                };

                write!(f, "{}", c)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn apply_shift(x: u64, delta: i32) -> u64 {
    if delta >= 0 {
        x << delta as u32
    } else {
        x >> (-delta) as u32
    }
}
