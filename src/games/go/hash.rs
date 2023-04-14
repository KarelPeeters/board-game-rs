use std::fmt::{Debug, Formatter};

use lazy_static::lazy_static;
use rand::distributions::{Distribution, Standard};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::board::Player;
use crate::games::go::{FlatTile, State, GO_MAX_AREA};

type Inner = u128;

#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Zobrist(Inner);

pub struct HashData {
    color_tile: [[Zobrist; GO_MAX_AREA as usize]; 2],
    color_turn: [Zobrist; 2],
    pass_state: [Zobrist; 3],
}

impl Zobrist {
    pub fn for_color_tile(color: Player, tile: FlatTile) -> Zobrist {
        HASH_DATA.color_tile[color.index() as usize][tile.index() as usize]
    }

    pub fn for_color_turn(color: Player) -> Zobrist {
        HASH_DATA.color_turn[color.index() as usize]
    }

    pub fn for_pass_state(state: State) -> Zobrist {
        // don't include outcome, that is implicit from the other tiles anyway
        let state_index = match state {
            State::Normal => 0,
            State::Passed => 1,
            State::Done(_) => 2,
        };
        HASH_DATA.pass_state[state_index]
    }
}

// TODO generate this at compile-time?
lazy_static! {
    static ref HASH_DATA: HashData = HashData::new();
}

impl HashData {
    #[allow(clippy::new_without_default)]
    pub fn new() -> HashData {
        let mut rng = StdRng::seed_from_u64(0);

        HashData {
            color_tile: [gen_array(&mut rng), gen_array(&mut rng)],
            color_turn: gen_array(&mut rng),
            pass_state: gen_array(&mut rng),
        }
    }
}

impl Debug for HashData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashData").finish_non_exhaustive()
    }
}

impl Distribution<Zobrist> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Zobrist {
        Zobrist(rng.gen())
    }
}

fn gen_array<T: Default + Copy, const N: usize>(rng: &mut impl Rng) -> [T; N]
where
    Standard: Distribution<T>,
{
    let mut array = [T::default(); N];
    for i in 0..N {
        array[i] = rng.gen();
    }
    array
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
