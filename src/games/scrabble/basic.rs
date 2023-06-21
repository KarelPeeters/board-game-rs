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

#[derive(Copy, Clone, Eq, PartialEq)]
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
            result.add(Letter::from_char(c)?);
        }
        Ok(result)
    }

    pub fn add(&mut self, c: Letter) {
        self.mask.set(c, true);
        self.counts[c.index as usize] += 1;
    }

    pub fn remove(&mut self, c: Letter) {
        debug_assert!(self.counts[c.index as usize] > 0 && self.mask.get(c));
        self.counts[c.index as usize] -= 1;
        if self.counts[c.index as usize] == 0 {
            self.mask.set(c, false);
        }
    }

    pub fn try_remove(&mut self, c: Letter) -> bool {
        if self.mask.get(c) {
            self.remove(c);
            true
        } else {
            false
        }
    }

    pub fn count(self) -> u8 {
        // TODO cache this?
        self.counts.iter().sum()
    }

    pub fn is_empty(self) -> bool {
        self.mask.is_empty()
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
