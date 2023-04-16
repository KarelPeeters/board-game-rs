use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use static_assertions::const_assert;

use crate::board::Player;
use crate::games::go::{FlatTile, State, GO_MAX_AREA};

#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Zobrist(Inner);

include!(concat!(env!("OUT_DIR"), "/go_hash_code.rs"));

const HASH_BYTES: usize = Inner::BITS as usize / 8;
const HASH_DATA_COLOR_TILE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/go_hash_data.bin"));

const_assert!(GO_HASH_COUNT == 2 * (GO_MAX_AREA as usize));
const_assert!(HASH_DATA_COLOR_TILE.len() == GO_HASH_COUNT * HASH_BYTES);

impl Zobrist {
    fn from_bytes_at(data: &[u8], index: usize) -> Self {
        let bytes = &data[index * HASH_BYTES..][..HASH_BYTES];
        let array = bytes.try_into().unwrap();
        let inner = Inner::from_ne_bytes(array);
        Zobrist(inner)
    }

    pub fn for_color_tile(color: Player, tile: FlatTile) -> Zobrist {
        // TODO try other way around to see if it's faster
        let index = color.index() as usize * GO_MAX_AREA as usize + tile.index() as usize;
        Zobrist::from_bytes_at(HASH_DATA_COLOR_TILE, index)
    }

    pub fn for_color_turn(color: Player) -> Zobrist {
        let index = color.index() as usize;
        Zobrist(HASH_DATA_TURN[index])
    }

    pub fn for_pass_state(state: State) -> Zobrist {
        // don't include outcome itself, that is implicit from the other tiles anyway
        let index = match state {
            State::Normal => 0,
            State::Passed => 1,
            State::Done(_) => 2,
        };
        Zobrist(HASH_DATA_PASS[index])
    }
}

impl Debug for Zobrist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // print hex, full-width with leading 0x
        write!(f, "Zobrist({:#0width$x})", self.0, width = HASH_BYTES * 2 + 2,)
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

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::board::{Outcome, Player};
    use crate::games::go::{FlatTile, State, Zobrist, GO_MAX_SIZE};

    #[test]
    fn unique() {
        let mut set = HashSet::new();

        for color in [Player::A, Player::B] {
            assert!(set.insert(Zobrist::for_color_turn(color)));
        }

        assert!(set.insert(Zobrist::for_pass_state(State::Normal)));
        assert!(set.insert(Zobrist::for_pass_state(State::Passed)));
        assert!(set.insert(Zobrist::for_pass_state(State::Done(Outcome::Draw))));

        for color in [Player::A, Player::B] {
            for tile in FlatTile::all(GO_MAX_SIZE) {
                assert!(set.insert(Zobrist::for_color_tile(color, tile)));
            }
        }
    }
}
