use std::fmt::{Debug, Formatter};

use lazy_static::lazy_static;
use rand::distributions::{Distribution, Standard};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::board::Player;
use crate::games::go::{Chains, Tile};

lazy_static! {
    pub static ref HASH_DATA: HashData = HashData::new();
}

pub type Zobrist = u128;

const MAX_AREA: usize = Chains::MAX_AREA as usize;

pub struct HashData {
    pub player_tile: [[Zobrist; MAX_AREA]; 2],
    pub player_turn: [Zobrist; 2],
}

impl HashData {
    #[allow(clippy::new_without_default)]
    pub fn new() -> HashData {
        let mut rng = StdRng::seed_from_u64(0);

        HashData {
            player_tile: [gen_array(&mut rng), gen_array(&mut rng)],
            player_turn: gen_array(&mut rng),
        }
    }

    pub fn get_player_tile(&self, player: Player, tile: Tile, size: u8) -> Zobrist {
        // TODO use size? or should we always use the max size here?
        let player_index = player.index() as usize;
        let tile_index = tile.index(size);
        self.player_tile[player_index][tile_index]
    }
}

impl Debug for HashData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashData").finish_non_exhaustive()
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
