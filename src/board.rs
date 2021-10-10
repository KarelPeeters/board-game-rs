use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::panic::{RefUnwindSafe, UnwindSafe};

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::symmetry::Symmetry;

/// The main trait of this crate. Represents the state of a game.
/// Each game implementation is supposed to provide it's own constructors to allow for customizable start positions.
pub trait Board: 'static + Debug + Display + Clone + Eq + Hash + Send + Sync + UnwindSafe + RefUnwindSafe
where
    for<'a> Self: BoardAvailableMoves<'a, Self>,
{
    /// The type used to represent moves on this board.
    type Move: Debug + Display + Eq + Ord + Hash + Copy + Send + Sync + UnwindSafe + RefUnwindSafe;

    /// The type used to represent board symmetries.
    type Symmetry: Symmetry;

    /// Whether the player who plays a move can lose by playing that move.
    /// Symbolically whether `b.won_by() == Some(Winner::Player(b.next_player()))` can ever be true.
    /// This may be pessimistic, returning `true` is always correct.
    fn can_lose_after_move() -> bool;

    /// Return the next player to make a move.
    /// If the board is done this is the player that did not play the last move for consistency.
    fn next_player(&self) -> Player;

    /// Return whether the given move is available. Panics if this board is done.
    fn is_available_move(&self, mv: Self::Move) -> bool;

    /// Pick a random move from the `available_moves` with a uniform distribution. Panics if this board is done.
    /// Can be overridden for better performance.
    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        let count = self.available_moves().count();
        let index = rng.gen_range(0..count);
        // SAFETY: unwrap is safe because the index is less than the
        // length of the iterator.
        self.available_moves().nth(index).unwrap()
    }

    /// Play the move `mv`, modifying this board.
    /// Panics if this board is done or if the move is not available or valid for this board.
    fn play(&mut self, mv: Self::Move);

    /// Clone this board, play `mv` on it and return the new board.
    /// Panics if this board is done or if the move is not available or valid for this board.
    fn clone_and_play(&self, mv: Self::Move) -> Self {
        let mut next = self.clone();
        next.play(mv);
        next
    }

    /// The outcome of this board, is `None` when this games is not done yet.
    fn outcome(&self) -> Option<Outcome>;

    /// Whether this games is done.
    fn is_done(&self) -> bool {
        self.outcome().is_some()
    }

    /// Map this board under the given symmetry.
    fn map(&self, sym: Self::Symmetry) -> Self;

    /// Map a move under the given symmetry.
    fn map_move(sym: Self::Symmetry, mv: Self::Move) -> Self::Move;
}

/// A helper trait to get the correct lifetimes for [BoardAvailableMoves::available_moves].
/// This is a workaround to get generic associated types, See <https://github.com/rust-lang/rust/issues/44265>.
pub trait BoardAvailableMoves<'a, B: Board> {
    type AllMoveIterator: InternalIterator<Item = B::Move>;
    type MoveIterator: InternalIterator<Item = B::Move>;

    /// All theoretically possible moves, for any possible board.
    /// Moves returned by `available_moves` will always be a subset of these moves.
    /// The order of these moves does not need to match the order from `available_moves`.
    fn all_possible_moves() -> Self::AllMoveIterator;

    /// Return an iterator over available moves, is always nonempty. No guarantees are made about the ordering except
    /// that it stays consistent when the board is not modified.
    /// Panics if this board is done.
    fn available_moves(&'a self) -> Self::MoveIterator;
}

/// One of the two players.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Player {
    A,
    B,
}

/// The absolute outcome for a game.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Outcome {
    WonBy(Player),
    Draw,
}

impl Player {
    pub const BOTH: [Player; 2] = [Player::A, Player::B];

    pub fn other(self) -> Player {
        match self {
            Player::A => Player::B,
            Player::B => Player::A,
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Player::A => 0,
            Player::B => 1,
        }
    }

    pub fn sign<V: num::One + std::ops::Neg<Output = V>>(self, pov: Player) -> V {
        if self == pov {
            V::one()
        } else {
            -V::one()
        }
    }
}

/// A helper struct that can be used to implement [Board::available_moves]
/// based on [Board::all_possible_moves] and [Board::is_available_move].
/// This may be a lot slower then directly generating the available moves.
#[derive(Debug)]
pub struct BruteforceMoveIterator<'a, B: Board> {
    board: &'a B,
}

impl<'a, B: Board> BruteforceMoveIterator<'a, B> {
    pub fn new(board: &'a B) -> Self {
        BruteforceMoveIterator { board }
    }
}

impl<'a, B: Board> InternalIterator for BruteforceMoveIterator<'a, B> {
    type Item = B::Move;

    fn find_map<R, F>(self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
        B::all_possible_moves().find_map(
            |mv: B::Move| {
                if self.board.is_available_move(mv) {
                    f(mv)
                } else {
                    None
                }
            },
        )
    }
}
