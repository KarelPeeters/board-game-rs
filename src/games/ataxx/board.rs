use std::cmp::Ordering;
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::board::{AllMovesIterator, AvailableMovesIterator, Board, BoardMoves, BoardSymmetry, Outcome, Player};
use crate::games::ataxx::{Coord, Move, Tiles};
use crate::symmetry::D4Symmetry;

pub const MAX_MOVES_SINCE_LAST_COPY: u8 = 100;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AtaxxBoard {
    pub(super) size: u8,
    pub(super) tiles_a: Tiles,
    pub(super) tiles_b: Tiles,
    pub(super) gaps: Tiles,
    pub(super) moves_since_last_copy: u8,
    pub(super) next_player: Player,
    pub(super) outcome: Option<Outcome>,
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
        AtaxxBoard {
            size,
            tiles_a: Tiles::coord(Coord::from_xy(0, corner)) | Tiles::coord(Coord::from_xy(corner, 0)),
            tiles_b: Tiles::coord(Coord::from_xy(0, 0)) | Tiles::coord(Coord::from_xy(corner, corner)),
            gaps: Tiles::empty(),
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: if size == 2 { Some(Outcome::Draw) } else { None },
        }
    }

    pub fn empty(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE, "size {} is too large", size);
        AtaxxBoard {
            size,
            tiles_a: Tiles::empty(),
            tiles_b: Tiles::empty(),
            gaps: Tiles::empty(),
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: Some(Outcome::Draw),
        }
    }

    pub fn contains_coord(&self, coord: Coord) -> bool {
        coord.x() < self.size && coord.y() < self.size
    }

    pub fn tile(&self, coord: Coord) -> Option<Player> {
        assert!(self.contains_coord(coord));

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

    pub fn tiles_a(&self) -> Tiles {
        self.tiles_a
    }

    pub fn tiles_b(&self) -> Tiles {
        self.tiles_b
    }

    pub fn gaps(&self) -> Tiles {
        self.gaps
    }

    pub fn free_tiles(&self) -> Tiles {
        (self.tiles_a | self.tiles_b | self.gaps).not(self.size)
    }

    /// Return whether the player with the given tiles has to pass, ie. cannot make a copy or jump move.
    fn must_pass(&self, tiles: Tiles) -> bool {
        let possible_targets = tiles.copy_targets(self.size) | tiles.jump_targets(self.size);
        (possible_targets & self.free_tiles()).is_empty()
    }

    pub fn tiles_pov(&self) -> (Tiles, Tiles) {
        match self.next_player() {
            Player::A => (self.tiles_a, self.tiles_b),
            Player::B => (self.tiles_b, self.tiles_a),
        }
    }

    fn tiles_pov_mut(&mut self) -> (&mut Tiles, &mut Tiles) {
        match self.next_player {
            Player::A => (&mut self.tiles_a, &mut self.tiles_b),
            Player::B => (&mut self.tiles_b, &mut self.tiles_a),
        }
    }

    /// Set the correct outcome based on the current tiles and gaps.
    pub(super) fn update_outcome(&mut self) {
        let a_empty = self.tiles_a.is_empty();
        let b_empty = self.tiles_b.is_empty();

        let a_pass = self.must_pass(self.tiles_a);
        let b_pass = self.must_pass(self.tiles_b);

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
        let mask = Tiles::full(self.size).not(8);
        assert!((self.tiles_a & mask).is_empty());
        assert!((self.tiles_b & mask).is_empty());
        assert!((self.gaps & mask).is_empty());
        assert!((self.tiles_a & self.tiles_b).is_empty());
        assert!((self.tiles_a & self.gaps).is_empty());
        assert!((self.tiles_b & self.gaps).is_empty());
        let mut clone = self.clone();
        clone.update_outcome();
        assert_eq!(self.outcome, clone.outcome);
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
            Move::Pass => self.must_pass(next_tiles),
            Move::Copy { to } => {
                self.contains_coord(to) && (self.free_tiles() & next_tiles.copy_targets(self.size)).has(to)
            }
            Move::Jump { from, to } => {
                self.contains_coord(from)
                    && self.contains_coord(to)
                    && self.free_tiles().has(to)
                    && next_tiles.has(from)
                    && from.distance(to) == 2
            }
        }
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        assert!(!self.is_done());

        let next_tiles = self.tiles_pov().0;
        let free_tiles = self.free_tiles();

        if self.must_pass(next_tiles) {
            return Move::Pass;
        }

        let copy_targets = self.free_tiles() & next_tiles.copy_targets(self.size);
        let jump_targets = free_tiles & next_tiles.jump_targets(self.size);

        let copy_count = copy_targets.count() as u32;
        let jump_count: u32 = jump_targets
            .into_iter()
            .map(|to| (next_tiles & Tiles::coord(to).jump_targets(self.size)).count() as u32)
            .sum();

        let index = rng.gen_range(0..(copy_count + jump_count));

        if index < copy_count {
            Move::Copy {
                to: copy_targets.get_nth(index),
            }
        } else {
            let mut left = index - copy_count;
            for to in jump_targets {
                let from = next_tiles & Tiles::coord(to).jump_targets(self.size);
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
        assert!(self.is_available_move(mv), "{:?} is not available", mv);

        let size = self.size;
        let (next_tiles, other_tiles) = self.tiles_pov_mut();

        let to = match mv {
            Move::Pass => {
                // we don't need to check whether the game is finished now because the other player is guaranteed to have
                //   a real move, since otherwise the game would have finished already
                self.next_player = self.next_player.other();
                return;
            }
            Move::Copy { to } => to,
            Move::Jump { from, to } => {
                *next_tiles &= Tiles::coord(from).not(size);
                to
            }
        };

        let to = Tiles::coord(to);
        let converted = *other_tiles & to.copy_targets(size);
        *next_tiles |= to | converted;
        *other_tiles &= converted.not(size);

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
            tiles_a: self.tiles_a.map(self.size, sym),
            tiles_b: self.tiles_b.map(self.size, sym),
            gaps: self.gaps.map(self.size, sym),
            moves_since_last_copy: self.moves_since_last_copy,
            next_player: self.next_player,
            outcome: self.outcome,
        }
    }

    fn map_move(&self, sym: Self::Symmetry, mv: Move) -> Move {
        match mv {
            Move::Pass => Move::Pass,
            Move::Copy { to } => Move::Copy {
                to: to.map(self.size, sym),
            },
            Move::Jump { from, to } => Move::Jump {
                from: from.map(self.size, sym),
                to: to.map(self.size, sym),
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
        f(Move::Pass)?;
        for to in Coord::all(AtaxxBoard::MAX_SIZE) {
            f(Move::Copy { to })?;
        }
        for to in Coord::all(AtaxxBoard::MAX_SIZE) {
            for from in Tiles::coord(to).jump_targets(8) {
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
        if board.must_pass(next_tiles) {
            return f(Move::Pass);
        }

        // copy moves
        let copy_targets = free_tiles & next_tiles.copy_targets(board.size);
        for to in copy_targets {
            f(Move::Copy { to })?;
        }

        // jump moves
        let jump_targets = free_tiles & next_tiles.jump_targets(board.size);
        for to in jump_targets {
            for from in next_tiles & Tiles::coord(to).jump_targets(board.size) {
                f(Move::Jump { from, to })?;
            }
        }

        ControlFlow::Continue(())
    }
}
