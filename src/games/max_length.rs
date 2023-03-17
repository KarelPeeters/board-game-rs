use std::fmt::{Display, Formatter};

use rand::Rng;

use crate::board::{Board, BoardDone, BoardMoves, BoardSymmetry, Outcome, PlayError, Player};

/// A wrapper around an existing board that has the same behaviour,
/// except that the outcome is a draw after a fixed number of moves has been played.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MaxMovesBoard<B: Board> {
    inner: B,
    moves: u64,
    max_moves: u64,
}

impl<B: Board> MaxMovesBoard<B> {
    pub fn new(inner: B, max_moves: u64) -> Self {
        MaxMovesBoard {
            inner,
            moves: 0,
            max_moves,
        }
    }

    pub fn inner(&self) -> &B {
        &self.inner
    }

    pub fn into_inner(self) -> B {
        self.inner
    }
}

impl<B: Board> Board for MaxMovesBoard<B> {
    type Move = B::Move;

    fn next_player(&self) -> Player {
        self.inner.next_player()
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;
        self.inner.is_available_move(mv)
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Result<Self::Move, BoardDone> {
        self.check_done()?;
        self.inner.random_available_move(rng)
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        // we don't use `check_can_play` here, the inner one will do that
        //   we still need to `check_done` though, since that might be different
        self.check_done()?;
        self.inner.play(mv)?;
        self.moves += 1;
        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.moves == self.max_moves {
            Some(Outcome::Draw)
        } else {
            self.inner.outcome()
        }
    }

    fn can_lose_after_move() -> bool {
        B::can_lose_after_move()
    }
}

impl<B: Board> BoardSymmetry<MaxMovesBoard<B>> for MaxMovesBoard<B> {
    type Symmetry = B::Symmetry;
    type CanonicalKey = B::CanonicalKey;

    fn map(&self, sym: Self::Symmetry) -> Self {
        MaxMovesBoard {
            inner: self.inner.map(sym),
            moves: self.moves,
            max_moves: self.max_moves,
        }
    }

    fn map_move(&self, sym: Self::Symmetry, mv: B::Move) -> B::Move {
        B::map_move(self.inner(), sym, mv)
    }

    fn canonical_key(&self) -> Self::CanonicalKey {
        self.inner.canonical_key()
    }
}

impl<'a, B: Board> BoardMoves<'a, MaxMovesBoard<B>> for MaxMovesBoard<B> {
    type AllMovesIterator = <B as BoardMoves<'a, B>>::AllMovesIterator;
    type AvailableMovesIterator = <B as BoardMoves<'a, B>>::AvailableMovesIterator;

    fn all_possible_moves() -> Self::AllMovesIterator {
        B::all_possible_moves()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        assert!(!self.is_done());
        self.inner.available_moves()
    }
}

impl<B: Board> Display for MaxMovesBoard<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\nmoves: {}/{:?}", self.inner, self.moves, self.max_moves)
    }
}
