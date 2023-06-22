use internal_iterator::InternalIterator;
use std::ops::ControlFlow;

use rand::Rng;

use crate::util::bits::BitIter;
use crate::util::iter::IterExt;

pub const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LETTER_COUNT: usize = LETTERS.len();
pub const LETTER_INFO: [LetterInfo; LETTER_COUNT] = [
    LetterInfo::new(1, 9),  // A
    LetterInfo::new(3, 2),  // B
    LetterInfo::new(3, 2),  // C
    LetterInfo::new(2, 4),  // D
    LetterInfo::new(1, 12), // E
    LetterInfo::new(4, 2),  // F
    LetterInfo::new(2, 3),  // G
    LetterInfo::new(4, 2),  // H
    LetterInfo::new(1, 9),  // I
    LetterInfo::new(8, 1),  // J
    LetterInfo::new(5, 1),  // K
    LetterInfo::new(1, 4),  // L
    LetterInfo::new(3, 2),  // M
    LetterInfo::new(1, 6),  // N
    LetterInfo::new(1, 8),  // O
    LetterInfo::new(3, 2),  // P
    LetterInfo::new(10, 1), // Q
    LetterInfo::new(1, 6),  // R
    LetterInfo::new(1, 4),  // S
    LetterInfo::new(1, 6),  // T
    LetterInfo::new(1, 4),  // U
    LetterInfo::new(4, 2),  // V
    LetterInfo::new(4, 2),  // W
    LetterInfo::new(8, 1),  // X
    LetterInfo::new(4, 2),  // Y
    LetterInfo::new(10, 1), // Z
];

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Letter {
    index: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LetterInfo {
    score: u8,
    initial_count: u8,
}

#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct Mask(u32);

pub const MAX_DECK_SIZE: usize = 7;

// TODO rename to rack
#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct Deck {
    // TODO is storing mask and [u8; MAX_DECK_SIZE] faster?
    mask: Mask,
    counts: [u8; LETTER_COUNT],
    // TODO add wildcards
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidLetter(char);

impl LetterInfo {
    pub const fn new(score: u8, initial_count: u8) -> Self {
        Self { score, initial_count }
    }
}

impl Letter {
    pub fn all() -> impl Iterator<Item = Letter> {
        LETTERS.bytes().map(|c| Letter::from_char(c as char).unwrap())
    }

    pub fn from_char(c: char) -> Result<Letter, InvalidLetter> {
        let c_upper = c.to_ascii_uppercase();
        if c_upper.is_ascii_uppercase() {
            Ok(Letter {
                index: c_upper as u8 - b'A',
            })
        } else {
            Err(InvalidLetter(c))
        }
    }

    pub fn from_index(index: u8) -> Letter {
        assert!((index as usize) < LETTER_COUNT);
        Letter { index }
    }

    pub fn to_ascii(self) -> u8 {
        self.index + b'A'
    }

    pub fn to_char(self) -> char {
        self.to_ascii() as char
    }

    pub fn to_mask(self) -> Mask {
        Mask(1 << self.index)
    }

    pub fn index(self) -> u8 {
        self.index
    }

    pub fn score_value(self) -> u8 {
        LETTER_INFO[self.index as usize].score
    }
}

impl Deck {
    pub fn from_letters(s: &str) -> Result<Deck, InvalidLetter> {
        assert!(s.len() <= MAX_DECK_SIZE);

        // TODO support wildcard
        let mut result = Deck::default();
        for c in s.chars() {
            result.add(Letter::from_char(c)?, 1);
        }
        Ok(result)
    }

    pub fn starting_bag() -> Deck {
        let mut result = Deck::default();
        for letter in Letter::all() {
            result.add(letter, LETTER_INFO[letter.index as usize].initial_count);
        }
        result
    }

    pub fn add(&mut self, c: Letter, count: u8) {
        self.mask.set(c, true);
        self.counts[c.index as usize] += count;
    }

    pub fn remove(&mut self, c: Letter, count: u8) {
        let index = c.index as usize;
        assert!(self.counts[index] >= count);

        self.counts[index] -= count;
        if self.counts[index] == 0 {
            self.mask.set(c, false);
        }
    }

    pub fn try_remove(&mut self, c: Letter, count: u8) -> bool {
        if self.counts[c.index() as usize] >= count {
            self.remove(c, count);
            true
        } else {
            false
        }
    }

    pub fn try_remove_all(&mut self, remove: Deck) -> bool {
        let mut copy = *self;

        for (c, count) in remove.letter_counts() {
            let curr = &mut copy.counts[c.index() as usize];

            if *curr < count {
                return false;
            }

            *curr -= count;
            if *curr == 0 {
                copy.mask.set(c, false);
            }
        }

        *self = copy;
        true
    }

    pub fn add_all(&mut self, other: Deck) {
        self.mask |= other.mask;
        for (c, count) in other.letter_counts() {
            self.counts[c.index() as usize] += count;
        }
    }

    pub fn tile_count(self) -> u8 {
        // TODO cache this?
        self.counts.iter().sum()
    }

    pub fn count_for(self, letter: Letter) -> u8 {
        self.counts[letter.index() as usize]
    }

    pub fn is_empty(self) -> bool {
        self.mask.is_empty()
    }

    pub fn usable_mask(self) -> Mask {
        self.mask
    }

    pub fn is_superset_of(self, other: Deck) -> bool {
        if !self.mask.is_superset_of(other.mask) {
            return false;
        }
        self.counts.iter().zip(other.counts.iter()).all(|(a, b)| a >= b)
    }

    pub fn letter_counts(self) -> impl Iterator<Item = (Letter, u8)> {
        self.mask
            .letters()
            .pure_map(move |c| (c, self.counts[c.index() as usize]))
    }

    pub fn assert_valid(self) {
        for c in Letter::all() {
            assert_eq!(
                self.counts[c.index() as usize] > 0,
                self.mask.get(c),
                "Mismatch for letter {c:?}"
            );
        }
        assert_eq!(self.tile_count(), self.counts.iter().copied().sum());
    }

    pub fn remove_sample(&mut self, rng: &mut impl Rng) -> Option<Letter> {
        let total_count = self.tile_count();
        if total_count == 0 {
            return None;
        }

        let index = rng.gen_range(0..total_count);

        let mut sum = 0;
        for (c, count) in self.letter_counts() {
            sum += count;
            if sum > index {
                self.remove(c, 1);
                return Some(c);
            }
        }

        unreachable!()
    }

    pub fn sub_decks(self, max_count: u8) -> SubDecks {
        SubDecks { deck: self, max_count }
    }
}

#[derive(Debug)]
pub struct SubDecks {
    deck: Deck,
    max_count: u8,
}

impl SubDecks {
    fn for_each_sub_deck_impl<R>(
        &self,
        skip: usize,
        mut curr: Deck,
        f: &mut impl FnMut(Deck) -> ControlFlow<R>,
    ) -> ControlFlow<R> {
        if cfg!(debug_assertions) {
            self.deck.assert_valid();
            curr.assert_valid();
        }

        if curr.tile_count() > self.max_count {
            return ControlFlow::Continue(());
        }

        if let Some((letter, count)) = self.deck.letter_counts().nth(skip) {
            debug_assert_eq!(curr.counts[letter.index() as usize], 0);

            // continue recursing on current letter
            for i in 0..=count {
                curr.counts[letter.index() as usize] = i;
                curr.mask.set(letter, i > 0);

                self.for_each_sub_deck_impl(skip + 1, curr, f)?;
            }
        } else {
            // end of the recursion
            f(curr)?;
        }

        ControlFlow::Continue(())
    }
}

impl InternalIterator for SubDecks {
    type Item = Deck;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        self.for_each_sub_deck_impl(0, Deck::default(), &mut f)
    }
}

impl Mask {
    pub const NONE: Mask = Mask(0);
    pub const ALL_LETTERS: Mask = Mask((1 << LETTER_COUNT) - 1);

    pub fn from_letters(s: &str) -> Result<Mask, InvalidLetter> {
        let mut result = Mask::NONE;
        for c in s.chars() {
            result.set(Letter::from_char(c)?, true);
        }
        Ok(result)
    }

    pub fn from_inner(inner: u32) -> Mask {
        assert_eq!(inner & !Self::ALL_LETTERS.inner(), 0);
        Mask(inner)
    }

    pub fn inner(self) -> u32 {
        self.0
    }

    pub fn get(self, c: Letter) -> bool {
        self.0 & (1 << c.index) != 0
    }

    pub fn set(&mut self, c: Letter, value: bool) {
        if value {
            self.0 |= 1 << c.index;
        } else {
            self.0 &= !(1 << c.index);
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn has_all_letters(self) -> bool {
        self & Self::ALL_LETTERS == Self::ALL_LETTERS
    }

    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub fn letters(self) -> impl Iterator<Item = Letter> {
        BitIter::new(self.0).map(|index| Letter { index })
    }

    pub fn is_superset_of(self, other: Mask) -> bool {
        self.0 & other.0 == other.0
    }
}

mod debug {
    use std::fmt::{Debug, Formatter};

    use super::*;

    impl Debug for Letter {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Letter('{}')", self.to_char())
        }
    }

    impl Debug for Mask {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            if *self == Mask::ALL_LETTERS {
                write!(f, "Mask(ALL_LETTERS)")
            } else {
                write!(f, "Mask(\"")?;
                for (i, c) in LETTERS.chars().enumerate() {
                    if self.0 & (1 << i) != 0 {
                        write!(f, "{}", c)?;
                    }
                }
                write!(f, "\")")
            }
        }
    }

    impl Debug for Deck {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Deck(\"")?;
            for (i, c) in LETTERS.chars().enumerate() {
                for _ in 0..self.counts[i] {
                    write!(f, "{}", c)?;
                }
            }
            write!(f, "\")")
        }
    }
}

mod operations {
    use super::*;

    impl std::ops::BitOr for Mask {
        type Output = Mask;

        fn bitor(self, rhs: Self) -> Self::Output {
            Mask(self.0 | rhs.0)
        }
    }

    impl std::ops::BitAnd for Mask {
        type Output = Mask;

        fn bitand(self, rhs: Self) -> Self::Output {
            Mask(self.0 & rhs.0)
        }
    }

    impl std::ops::BitXor for Mask {
        type Output = Mask;

        fn bitxor(self, rhs: Self) -> Self::Output {
            Mask(self.0 ^ rhs.0)
        }
    }

    impl std::ops::BitOrAssign for Mask {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0
        }
    }

    impl std::ops::BitAndAssign for Mask {
        fn bitand_assign(&mut self, rhs: Self) {
            self.0 &= rhs.0
        }
    }

    impl std::ops::BitXorAssign for Mask {
        fn bitxor_assign(&mut self, rhs: Self) {
            self.0 ^= rhs.0
        }
    }
}
