use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use lazy_static::lazy_static;
use rand::distributions::{Distribution, Standard};
use rand::Rng;

use crate::games::scrabble::basic::{Deck, Letter, LETTER_COUNT, MAX_DECK_SIZE};
use crate::games::scrabble::grid::ScrabbleGrid;
use crate::util::tiny::consistent_rng;

type Inner = u128;

#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Zobrist(Inner);

pub struct ZobristData {
    grid_letter: Vec<Zobrist>,
    deck_letter: [Vec<Zobrist>; 2],
    grid_start: Vec<Zobrist>,

    exchange_counter: [Zobrist; 4],
}

impl Zobrist {
    pub fn for_grid_letter(x: u8, y: u8, letter: Letter) -> Zobrist {
        assert!(x < ScrabbleGrid::MAX_SIZE && y < ScrabbleGrid::MAX_SIZE);
        let max_size = ScrabbleGrid::MAX_SIZE as usize;
        let index = y as usize * max_size * LETTER_COUNT + x as usize * LETTER_COUNT + letter.index() as usize;
        ZOBRIST_DATA.grid_letter[index]
    }

    pub fn for_deck_letter_count(pov: bool, letter: Letter, count: u8) -> Zobrist {
        let count = count as usize;
        assert!(0 < count && count <= MAX_DECK_SIZE);
        let index = letter.index() as usize * MAX_DECK_SIZE + count;
        ZOBRIST_DATA.deck_letter[pov as usize][index]
    }

    pub fn for_grid_start(x: u8, y: u8) -> Zobrist {
        assert!(x < ScrabbleGrid::MAX_SIZE && y < ScrabbleGrid::MAX_SIZE);
        let max_size = ScrabbleGrid::MAX_SIZE as usize;
        let index = y as usize * max_size + x as usize;
        ZOBRIST_DATA.grid_start[index]
    }

    pub fn for_exchange_count(counter: u8) -> Zobrist {
        ZOBRIST_DATA.exchange_counter[counter as usize]
    }

    pub fn for_deck(pov: bool, deck: Deck) -> Zobrist {
        let mut result = Zobrist::default();
        // TODO wildcard
        for letter in deck.usable_mask().letters() {
            result ^= Zobrist::for_deck_letter_count(pov, letter, deck.count_for(letter))
        }
        result
    }

    pub fn inner(self) -> Inner {
        self.0
    }
}

// TODO generate this at compile-time?
lazy_static! {
    static ref ZOBRIST_DATA: ZobristData = ZobristData::new();
}

impl ZobristData {
    #[allow(clippy::new_without_default)]
    #[inline(never)]
    pub fn new() -> ZobristData {
        let mut rng = consistent_rng();

        let max_size = ScrabbleGrid::MAX_SIZE as usize;

        ZobristData {
            grid_letter: gen_vec(max_size * max_size * LETTER_COUNT, &mut rng),
            deck_letter: [
                gen_vec(LETTER_COUNT * MAX_DECK_SIZE, &mut rng),
                gen_vec(LETTER_COUNT * MAX_DECK_SIZE, &mut rng),
            ],
            grid_start: gen_vec(max_size * max_size, &mut rng),
            exchange_counter: gen_array(&mut rng),
        }
    }
}

impl Debug for ZobristData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashData").finish_non_exhaustive()
    }
}

impl Distribution<Zobrist> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Zobrist {
        Zobrist(rng.gen())
    }
}

fn gen_array<const N: usize>(rng: &mut impl Rng) -> [Zobrist; N] {
    let mut array = [Zobrist::default(); N];
    for x in &mut array {
        *x = rng.gen();
    }
    array
}

fn gen_vec(len: usize, rng: &mut impl Rng) -> Vec<Zobrist> {
    Standard.sample_iter(rng).take(len).collect()
}

impl Debug for Zobrist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // print hex, full-width with leading 0x
        write!(
            f,
            "Zobrist({:#0width$x})",
            self.0,
            width = (Inner::BITS / 8 + 2) as usize
        )
    }
}

impl std::ops::BitXor for Zobrist {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Zobrist(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for Zobrist {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl nohash_hasher::IsEnabled for Zobrist {}

impl Hash for Zobrist {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64((self.0 as u64) ^ ((self.0 >> 64) as u64));
    }
}
