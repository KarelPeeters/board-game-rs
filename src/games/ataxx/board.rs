use std::cmp::Ordering;
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::board::{
    AllMovesIterator, Alternating, AvailableMovesIterator, Board, BoardDone, BoardMoves, BoardSymmetry, Outcome,
    PlayError, Player,
};
use crate::symmetry::D4Symmetry;
use crate::util::bitboard::BitBoard8;
use crate::util::coord::Coord8;

pub const MAX_MOVES_SINCE_LAST_COPY: u8 = 100;

// TODO in theory we can remove size and just add more gaps
//   maybe an issue for move encoding, although it can also just check the gaps mask

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AtaxxBoard {
    pub(super) size: u8,
    pub(super) tiles_a: BitBoard8,
    pub(super) tiles_b: BitBoard8,
    pub(super) gaps: BitBoard8,
    pub(super) moves_since_last_copy: u8,
    pub(super) next_player: Player,
    pub(super) outcome: Option<Outcome>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Move {
    Pass,
    Copy { to: Coord8 },
    Jump { from: Coord8, to: Coord8 },
}

impl Default for AtaxxBoard {
    fn default() -> Self {
        AtaxxBoard::diagonal(7)
    }
}

impl AtaxxBoard {
    pub const MAX_SIZE: u8 = 8;

    pub fn from_parts(
        size: u8,
        tiles_a: BitBoard8,
        tiles_b: BitBoard8,
        gaps: BitBoard8,
        moves_since_last_copy: u8,
        next_player: Player,
    ) -> Self {
        let mut result = AtaxxBoard {
            size,
            tiles_a,
            tiles_b,
            gaps,
            moves_since_last_copy,
            next_player,
            outcome: None,
        };
        result.update_outcome();
        result.assert_valid();
        result
    }

    pub fn diagonal(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE, "size {} is too large", size);
        assert!(size >= 2, "diagonal board is only possible with size > 2, got {}", size);

        let corner = size - 1;
        let tiles_a = BitBoard8::coord(Coord8::from_xy(0, corner)) | BitBoard8::coord(Coord8::from_xy(corner, 0));
        let tiles_b = BitBoard8::coord(Coord8::from_xy(0, 0)) | BitBoard8::coord(Coord8::from_xy(corner, corner));

        AtaxxBoard {
            size,
            tiles_a,
            tiles_b,
            gaps: BitBoard8::EMPTY,
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: if size == 2 { Some(Outcome::Draw) } else { None },
        }
    }

    pub fn empty(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE, "size {} is too large", size);
        AtaxxBoard {
            size,
            tiles_a: BitBoard8::EMPTY,
            tiles_b: BitBoard8::EMPTY,
            gaps: BitBoard8::EMPTY,
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: Some(Outcome::Draw),
        }
    }

    pub fn valid_coord(&self, coord: Coord8) -> bool {
        coord.valid_for_size(self.size)
    }

    pub fn full_mask(&self) -> BitBoard8 {
        BitBoard8::FULL_FOR_SIZE[self.size as usize]
    }

    pub fn tile(&self, coord: Coord8) -> Option<Player> {
        assert!(self.valid_coord(coord));

        if self.tiles_a.has(coord) {
            return Some(Player::A);
        }
        if self.tiles_b.has(coord) {
            return Some(Player::B);
        }
        None
    }

    pub fn clear_moves_since_last_copy(&mut self) {
        self.moves_since_last_copy = 0;
    }

    pub fn moves_since_last_copy(&self) -> u8 {
        self.moves_since_last_copy
    }

    pub fn size(&self) -> u8 {
        self.size
    }

    pub fn tiles_a(&self) -> BitBoard8 {
        self.tiles_a
    }

    pub fn tiles_b(&self) -> BitBoard8 {
        self.tiles_b
    }

    pub fn gaps(&self) -> BitBoard8 {
        self.gaps
    }

    pub fn free_tiles(&self) -> BitBoard8 {
        !(self.tiles_a | self.tiles_b | self.gaps) & self.full_mask()
    }

    /// Returns whether the current played must pass.
    /// Returns false if the game is already done.
    pub fn must_pass(&self) -> bool {
        !self.is_done() && self.must_pass_with_tiles(self.tiles_pov().0)
    }

    /// Return whether the player with the given tiles has to pass, ie. cannot make a copy or jump move.
    pub(super) fn must_pass_with_tiles(&self, tiles: BitBoard8) -> bool {
        (tiles.adjacent_or_ring_not_self() & self.free_tiles()).none()
    }

    pub fn tiles_pov(&self) -> (BitBoard8, BitBoard8) {
        match self.next_player() {
            Player::A => (self.tiles_a, self.tiles_b),
            Player::B => (self.tiles_b, self.tiles_a),
        }
    }

    pub(super) fn tiles_pov_mut(&mut self) -> (&mut BitBoard8, &mut BitBoard8) {
        match self.next_player {
            Player::A => (&mut self.tiles_a, &mut self.tiles_b),
            Player::B => (&mut self.tiles_b, &mut self.tiles_a),
        }
    }

    /// Set the correct outcome based on the current tiles and gaps.
    pub(super) fn update_outcome(&mut self) {
        let a_empty = self.tiles_a.none();
        let b_empty = self.tiles_b.none();

        let a_pass = self.must_pass_with_tiles(self.tiles_a);
        let b_pass = self.must_pass_with_tiles(self.tiles_b);

        let outcome = if self.moves_since_last_copy >= MAX_MOVES_SINCE_LAST_COPY || (a_empty && b_empty) {
            Some(Outcome::Draw)
        } else if a_empty {
            Some(Outcome::WonBy(Player::B))
        } else if b_empty {
            Some(Outcome::WonBy(Player::A))
        } else if a_pass && b_pass {
            let count_a = self.tiles_a.count();
            let count_b = self.tiles_b.count();

            let outcome = match count_a.cmp(&count_b) {
                Ordering::Less => Outcome::WonBy(Player::B),
                Ordering::Equal => Outcome::Draw,
                Ordering::Greater => Outcome::WonBy(Player::A),
            };
            Some(outcome)
        } else {
            None
        };

        self.outcome = outcome;
    }

    pub fn assert_valid(&self) {
        // no invalid bits
        let invalid_mask = !self.full_mask();
        assert!((self.tiles_a & invalid_mask).none());
        assert!((self.tiles_b & invalid_mask).none());
        assert!((self.gaps & invalid_mask).none());

        // no overlapping tiles
        assert!((self.tiles_a & self.tiles_b).none());
        assert!((self.tiles_a & self.gaps).none());
        assert!((self.tiles_b & self.gaps).none());

        // correct outcome
        let mut clone = self.clone();
        clone.update_outcome();
        assert_eq!(self.outcome, clone.outcome);
    }

    pub fn map_coord(&self, coord: Coord8, sym: D4Symmetry) -> Coord8 {
        assert!(self.valid_coord(coord));
        sym.map_coord(coord, self.size)
    }

    pub fn map_tiles(&self, tiles: BitBoard8, sym: D4Symmetry) -> BitBoard8 {
        // TODO do this with some proper bit operations
        //   careful: it's not as simple as just using the BitBoard functions, since we need to support smaller sizes
        let mut result = BitBoard8::EMPTY;
        for coord in tiles {
            let result_coord = self.map_coord(coord, sym);
            result |= BitBoard8::coord(result_coord);
        }
        result
    }
}

impl Move {
    pub fn valid_for_size(self, size: u8) -> bool {
        match self {
            Move::Pass => true,
            Move::Copy { to } => to.valid_for_size(size),
            Move::Jump { from, to } => {
                from.valid_for_size(size) && to.valid_for_size(size) && from.diagonal_distance(to) == 2
            }
        }
    }
}

impl Board for AtaxxBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        if !mv.valid_for_size(self.size) {
            return Ok(false);
        }

        let next_tiles = self.tiles_pov().0;

        let result = match mv {
            Move::Pass => self.must_pass_with_tiles(next_tiles),
            Move::Copy { to } => (self.free_tiles() & next_tiles.adjacent()).has(to),
            Move::Jump { from, to } => self.free_tiles().has(to) && next_tiles.has(from),
        };
        Ok(result)
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Result<Self::Move, BoardDone> {
        self.check_done()?;

        let next_tiles = self.tiles_pov().0;
        let free_tiles = self.free_tiles();

        if self.must_pass_with_tiles(next_tiles) {
            return Ok(Move::Pass);
        }

        let copy_targets = free_tiles & next_tiles.adjacent();
        let jump_targets = free_tiles & next_tiles.ring();

        let copy_count = copy_targets.count() as u32;
        let jump_count: u32 = jump_targets
            .into_iter()
            .map(|to| (next_tiles & coord_to_ring(to)).count() as u32)
            .sum();

        let index = rng.gen_range(0..(copy_count + jump_count));

        if index < copy_count {
            Ok(Move::Copy {
                to: copy_targets.get_nth(index),
            })
        } else {
            let mut left = index - copy_count;
            for to in jump_targets {
                let from = next_tiles & coord_to_ring(to);
                let count = from.count() as u32;
                if left < count {
                    let from = from.get_nth(left);
                    return Ok(Move::Jump { from, to });
                }
                left -= count;
            }

            unreachable!()
        }
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_can_play(mv)?;

        let (next_tiles, other_tiles) = self.tiles_pov_mut();

        let to = match mv {
            Move::Pass => {
                // we don't need to check whether the game is finished now because the other player is guaranteed to have
                //   a real move, since otherwise the game would have finished already
                self.next_player = self.next_player.other();
                self.moves_since_last_copy += 1;
                return Ok(());
            }
            Move::Copy { to } => to,
            Move::Jump { from, to } => {
                *next_tiles &= !BitBoard8::coord(from);
                to
            }
        };

        let to = BitBoard8::coord(to);
        let converted = *other_tiles & to.adjacent();
        *next_tiles |= to | converted;
        *other_tiles &= !converted;

        self.moves_since_last_copy += 1;
        if let Move::Copy { .. } = mv {
            self.moves_since_last_copy = 0;
        }

        self.update_outcome();
        self.next_player = self.next_player.other();

        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl Alternating for AtaxxBoard {}

impl BoardSymmetry<AtaxxBoard> for AtaxxBoard {
    type Symmetry = D4Symmetry;
    type CanonicalKey = (u64, u64, u64);

    fn map(&self, sym: Self::Symmetry) -> Self {
        AtaxxBoard {
            size: self.size,
            tiles_a: self.map_tiles(self.tiles_a, sym),
            tiles_b: self.map_tiles(self.tiles_b, sym),
            gaps: self.map_tiles(self.gaps, sym),
            moves_since_last_copy: self.moves_since_last_copy,
            next_player: self.next_player,
            outcome: self.outcome,
        }
    }

    fn map_move(&self, sym: Self::Symmetry, mv: Move) -> Move {
        match mv {
            Move::Pass => Move::Pass,
            Move::Copy { to } => Move::Copy {
                to: self.map_coord(to, sym),
            },
            Move::Jump { from, to } => Move::Jump {
                from: self.map_coord(from, sym),
                to: self.map_coord(to, sym),
            },
        }
    }

    fn canonical_key(&self) -> Self::CanonicalKey {
        (self.tiles_a.0, self.tiles_b.0, self.gaps.0)
    }
}

impl<'a> BoardMoves<'a, AtaxxBoard> for AtaxxBoard {
    type AllMovesIterator = AllMovesIterator<AtaxxBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, AtaxxBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        AvailableMovesIterator::new(self)
    }
}

impl InternalIterator for AllMovesIterator<AtaxxBoard> {
    type Item = Move;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        let full_board = BitBoard8::FULL_FOR_SIZE[AtaxxBoard::MAX_SIZE as usize];

        f(Move::Pass)?;
        for to in full_board {
            f(Move::Copy { to })?;
        }
        for to in full_board {
            for from in coord_to_ring(to) & full_board {
                f(Move::Jump { from, to })?;
            }
        }

        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, AtaxxBoard> {
    type Item = Move;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        let board = self.board();
        let next_tiles = board.tiles_pov().0;
        let free_tiles = board.free_tiles();

        // pass move, don't emit other moves afterwards
        if board.must_pass_with_tiles(next_tiles) {
            return f(Move::Pass);
        }

        // copy moves
        let copy_targets = free_tiles & next_tiles.adjacent();
        for to in copy_targets {
            f(Move::Copy { to })?;
        }

        // jump moves
        let jump_targets = free_tiles & next_tiles.ring();
        for to in jump_targets {
            for from in next_tiles & coord_to_ring(to) {
                f(Move::Jump { from, to })?;
            }
        }

        ControlFlow::Continue(())
    }

    fn count(self) -> usize {
        let board = self.board();
        let next_tiles = board.tiles_pov().0;
        let free_tiles = board.free_tiles();

        if board.must_pass_with_tiles(next_tiles) {
            return 1;
        }

        let mut count = 0;

        let copy_targets = free_tiles & next_tiles.adjacent();
        count += copy_targets.count() as usize;

        for from in next_tiles {
            let to = coord_to_ring(from) & free_tiles;
            count += to.count() as usize;
        }

        count
    }
}

/// The same as `BitBoard8::coord(from).ring()` but hopefully faster.
pub fn coord_to_ring(coord: Coord8) -> BitBoard8 {
    macro_rules! coord_to_ring_values {
        [$($index:literal),+] => {
            [$(BitBoard8::coord(Coord8::from_index($index)).ring()),+]
        }
    }
    #[rustfmt::skip]
    const COORD_TO_RING_VALUES: [BitBoard8; 64] = coord_to_ring_values![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63
    ];
    COORD_TO_RING_VALUES[coord.index() as usize]
}

#[cfg(test)]
mod tests {
    use crate::games::ataxx::coord_to_ring;
    use crate::util::bitboard::BitBoard8;
    use crate::util::coord::Coord8;

    #[test]
    fn ring() {
        for coord in Coord8::all() {
            assert_eq!(BitBoard8::coord(coord).ring(), coord_to_ring(coord));
        }
    }
}
