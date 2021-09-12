use std::fmt::Debug;

use crate::board::Board;

pub mod mcts;
pub mod minimax;
pub mod simple;
pub mod solver;

pub trait Bot<B: Board>: Debug {
    /// Pick a move to play. Panics if the board is done.
    ///
    /// `self` is mutable to allow for random state, this method is not supposed to
    /// modify `self` in any other significant way.
    fn select_move(&mut self, board: &B) -> B::Move;
}

impl<B: Board, F: FnMut(&B) -> B::Move + Debug> Bot<B> for F {
    fn select_move(&mut self, board: &B) -> B::Move {
        self(board)
    }
}
