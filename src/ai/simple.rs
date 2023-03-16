//! Two simple bots: `RandomBot` and `RolloutBot`.
use std::fmt::{Debug, Formatter};

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::ai::Bot;
use crate::board::{Board, BoardDone};
use crate::pov::NonPov;

/// Bot that chooses moves randomly uniformly among possible moves.
pub struct RandomBot<R: Rng> {
    rng: R,
}

impl<R: Rng> Debug for RandomBot<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RandomBot")
    }
}

impl<R: Rng> RandomBot<R> {
    pub fn new(rng: R) -> Self {
        RandomBot { rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for RandomBot<R> {
    fn select_move(&mut self, board: &B) -> Result<B::Move, BoardDone> {
        board.random_available_move(&mut self.rng)
    }
}

/// Bot that chooses moves after simulating random games for each of them.
///
/// The same number of simulations `rollouts / nb_moves` is done for
/// each move, and the move resulting in the best average score is selected.
pub struct RolloutBot<R: Rng> {
    rollouts: u32,
    rng: R,
}

impl<R: Rng> Debug for RolloutBot<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RolloutBot {{ rollouts: {} }}", self.rollouts)
    }
}

impl<R: Rng> RolloutBot<R> {
    pub fn new(rollouts: u32, rng: R) -> Self {
        RolloutBot { rollouts, rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for RolloutBot<R> {
    fn select_move(&mut self, board: &B) -> Result<B::Move, BoardDone> {
        let rollouts_per_move = self.rollouts / board.available_moves().unwrap().count() as u32;

        Ok(board
            .children()?
            .max_by_key(|(_, child)| {
                let score: i64 = (0..rollouts_per_move)
                    .map(|_| {
                        let mut copy = child.clone();
                        while let Ok(mv) = copy.random_available_move(&mut self.rng) {
                            copy.play(mv).unwrap();
                        }
                        copy.outcome().unwrap().pov(board.next_player()).sign::<i64>()
                    })
                    .sum();
                score
            })
            .map(|(mv, _)| mv)
            .unwrap())
    }
}
