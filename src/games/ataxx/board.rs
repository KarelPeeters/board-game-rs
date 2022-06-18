use std::cmp::Ordering;
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::board::{AllMovesIterator, AvailableMovesIterator, Board, BoardMoves, BoardSymmetry, Outcome, Player};
use crate::symmetry::D4Symmetry;
use crate::util::bitboard::BitBoard8;
use crate::util::coord::Coord8;

pub const MAX_MOVES_SINCE_LAST_COPY: u8 = 100;

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
        coord.x() < self.size && coord.y() < self.size
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
    fn must_pass_with_tiles(&self, tiles: BitBoard8) -> bool {
        let possible_targets = (tiles.adjacent() | tiles.ring()) & self.full_mask();
        (possible_targets & self.free_tiles()).none()
    }

    pub fn tiles_pov(&self) -> (BitBoard8, BitBoard8) {
        match self.next_player() {
            Player::A => (self.tiles_a, self.tiles_b),
            Player::B => (self.tiles_b, self.tiles_a),
        }
    }

    fn tiles_pov_mut(&mut self) -> (&mut BitBoard8, &mut BitBoard8) {
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
        let invalid_mask = !self.full_mask();
        assert!((self.tiles_a & invalid_mask).none());
        assert!((self.tiles_b & invalid_mask).none());
        assert!((self.gaps & invalid_mask).none());
        assert!((self.tiles_a & self.tiles_b).none());
        assert!((self.tiles_a & self.gaps).none());
        assert!((self.tiles_b & self.gaps).none());
        let mut clone = self.clone();
        clone.update_outcome();
        assert_eq!(self.outcome, clone.outcome);
    }

    pub fn map_coord(&self, coord: Coord8, sym: D4Symmetry) -> Coord8 {
        assert!(self.valid_coord(coord));
        sym.map_coord(coord, self.size)
    }

    pub fn map_tiles(&self, tiles: BitBoard8, sym: D4Symmetry) -> BitBoard8 {
        let mut result = BitBoard8::EMPTY;
        for coord in tiles {
            let result_coord = self.map_coord(coord, sym);
            result |= BitBoard8::coord(result_coord);

            println!("After mapping {} to {}, got tiles \n{}", coord, result_coord, result);
        }
        result
    }
}

impl Board for AtaxxBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());

        let next_tiles = self.tiles_pov().0;

        match mv {
            Move::Pass => self.must_pass_with_tiles(next_tiles),
            Move::Copy { to } => self.valid_coord(to) && (self.free_tiles() & next_tiles.adjacent()).has(to),
            Move::Jump { from, to } => {
                self.valid_coord(from)
                    && self.valid_coord(to)
                    && self.free_tiles().has(to)
                    && next_tiles.has(from)
                    && from.diagonal_distance(to) == 2
            }
        }
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        assert!(!self.is_done());

        let next_tiles = self.tiles_pov().0;
        let free_tiles = self.free_tiles();

        if self.must_pass_with_tiles(next_tiles) {
            return Move::Pass;
        }

        let copy_targets = free_tiles & next_tiles.adjacent();
        let jump_targets = free_tiles & next_tiles.ring();

        let copy_count = copy_targets.count() as u32;
        let jump_count: u32 = jump_targets
            .into_iter()
            .map(|to| (next_tiles & BitBoard8::coord(to).ring()).count() as u32)
            .sum();

        let index = rng.gen_range(0..(copy_count + jump_count));

        if index < copy_count {
            Move::Copy {
                to: copy_targets.get_nth(index),
            }
        } else {
            let mut left = index - copy_count;
            for to in jump_targets {
                let from = next_tiles & BitBoard8::coord(to).ring();
                let count = from.count() as u32;
                if left < count {
                    let from = from.get_nth(left);
                    return Move::Jump { from, to };
                }
                left -= count;
            }

            unreachable!()
        }
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(self.is_available_move(mv), "{:?} is not available on {:?}", mv, self);

        let (next_tiles, other_tiles) = self.tiles_pov_mut();

        let to = match mv {
            Move::Pass => {
                // we don't need to check whether the game is finished now because the other player is guaranteed to have
                //   a real move, since otherwise the game would have finished already
                self.next_player = self.next_player.other();
                self.moves_since_last_copy += 1;
                return;
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
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl BoardSymmetry<AtaxxBoard> for AtaxxBoard {
    type Symmetry = D4Symmetry;

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
}

impl<'a> BoardMoves<'a, AtaxxBoard> for AtaxxBoard {
    type AllMovesIterator = AllMovesIterator<AtaxxBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, AtaxxBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
        assert!(!self.is_done());
        AvailableMovesIterator(self)
    }
}

impl<'a> InternalIterator for AllMovesIterator<AtaxxBoard> {
    type Item = Move;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        let full_board = BitBoard8::FULL_FOR_SIZE[AtaxxBoard::MAX_SIZE as usize];

        f(Move::Pass)?;
        for to in full_board {
            f(Move::Copy { to })?;
        }
        for to in full_board {
            for from in BitBoard8::coord(to).ring() & full_board {
                f(Move::Jump { from, to })?;
            }
        }

        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, AtaxxBoard> {
    type Item = Move;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        let board = self.0;
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
            for from in next_tiles & BitBoard8::coord(to).ring() {
                f(Move::Jump { from, to })?;
            }
        }

        ControlFlow::Continue(())
    }
}
