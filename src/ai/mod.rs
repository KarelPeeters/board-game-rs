use crate::board::{Board, BoardDone};

pub mod mcts;
pub mod minimax;
pub mod simple;
pub mod solver;

pub trait Bot<B: Board> {
    /// Pick a move to play.
    ///
    /// `self` is mutable to allow for random state, this method is not supposed to
    /// modify `self` in any other significant way.
    fn select_move(&mut self, board: &B) -> Result<B::Move, BoardDone>;
}

impl<B: Board, F: FnMut(&B) -> Result<B::Move, BoardDone>> Bot<B> for F {
    fn select_move(&mut self, board: &B) -> Result<B::Move, BoardDone> {
        self(board)
    }
}
