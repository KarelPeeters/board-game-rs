use std::fmt::{Debug, Formatter};
use std::ops::Neg;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::ai::minimax::{minimax, minimax_value, Heuristic, MinimaxResult};
use crate::ai::Bot;
use crate::board::{Board, Outcome};
use crate::wdl::{OutcomeWDL, POV};

/// Minimax [Heuristic] that only looks at board outcomes.
/// When there are multiple winning moves it picks the shortest one,
/// and when there are only losing moves it picks the longest one.  
#[derive(Debug)]
pub struct SolverHeuristic;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SolverValue {
    WinIn(u32),
    LossIn(u32),
    Draw,
    Unknown,
}

impl<B: Board> Heuristic<B> for SolverHeuristic {
    type V = SolverValue;

    fn value(&self, board: &B, length: u32) -> SolverValue {
        board
            .outcome()
            .map_or(SolverValue::Unknown, |p| match p.pov(board.next_player()) {
                OutcomeWDL::Win => SolverValue::WinIn(length),
                OutcomeWDL::Draw => SolverValue::Draw,
                OutcomeWDL::Loss => SolverValue::LossIn(length),
            })
    }

    fn merge(old: SolverValue, new: SolverValue) -> (SolverValue, bool) {
        SolverValue::merge(old, new)
    }
}

impl SolverValue {
    pub fn to_i32(self) -> i32 {
        match self {
            SolverValue::WinIn(n) => i32::MAX - n as i32,
            SolverValue::LossIn(n) => -i32::MAX + n as i32,
            SolverValue::Draw | SolverValue::Unknown => 0,
        }
    }

    pub fn to_outcome_wdl(self) -> Option<OutcomeWDL> {
        match self {
            SolverValue::WinIn(_) => Some(OutcomeWDL::Win),
            SolverValue::LossIn(_) => Some(OutcomeWDL::Loss),
            SolverValue::Draw => Some(OutcomeWDL::Draw),
            SolverValue::Unknown => None,
        }
    }

    /// Return `(best_value, new >= old)`,
    /// where best_value properly accounts for shortest win, longest loss and draw vs unknown.
    pub fn merge(old: SolverValue, new: SolverValue) -> (SolverValue, bool) {
        use SolverValue::*;

        match (old, new) {
            // prefer shortest win and longest loss
            (WinIn(old_n), WinIn(new_n)) => (if new_n <= old_n { new } else { old }, new_n <= old_n),
            (LossIn(old_n), LossIn(new_n)) => (if new_n >= old_n { new } else { old }, new_n >= old_n),

            // win/loss is better/worse then everything else
            (WinIn(_), _) => (old, false),
            (LossIn(_), _) => (new, true),
            (_, WinIn(_)) => (new, true),
            (_, LossIn(_)) => (old, false),

            // draw and unknown are the same, but return unknown if either is unknown
            (Draw, Draw) => (Draw, true),
            (Unknown | Draw, Unknown | Draw) => (Unknown, true),
        }
    }

    /// Return whether `left >= right`.
    pub fn gte(left: SolverValue, right: SolverValue) -> bool {
        SolverValue::merge(right, left).1
    }

    /// Return whether `child` could a child of the optimally combined `parent`.
    pub fn could_be_optimal_child(parent: SolverValue, child: SolverValue) -> bool {
        let best_child = match parent {
            SolverValue::WinIn(n) => SolverValue::LossIn(n - 1),
            SolverValue::LossIn(n) => SolverValue::WinIn(n - 1),
            SolverValue::Draw => SolverValue::Draw,
            SolverValue::Unknown => panic!("This function does not work for unknown values"),
        };

        SolverValue::gte(child, best_child)
    }
}

impl Neg for SolverValue {
    type Output = SolverValue;

    fn neg(self) -> Self::Output {
        match self {
            SolverValue::WinIn(n) => SolverValue::LossIn(n),
            SolverValue::LossIn(n) => SolverValue::WinIn(n),
            SolverValue::Draw => SolverValue::Draw,
            SolverValue::Unknown => SolverValue::Unknown,
        }
    }
}

pub fn solve<B: Board>(board: &B, depth: u32, rng: &mut impl Rng) -> MinimaxResult<SolverValue, B::Move> {
    minimax(board, &SolverHeuristic, depth, rng)
}

pub fn solve_value<B: Board>(board: &B, depth: u32) -> SolverValue {
    minimax_value(board, &SolverHeuristic, depth)
}

/// Return whether this board is a double forced draw, ie. no matter what either player does the game can only end in a draw.
/// Returns `None` if the result is unknown.
pub fn is_double_forced_draw(board: &impl Board, depth: u32) -> Option<bool> {
    if let Some(outcome) = board.outcome() {
        return Some(outcome == Outcome::Draw);
    }
    if depth == 0 {
        return None;
    }

    let mut unknown = false;
    let draw_or_unknown = board.available_moves().all(|mv| {
        let child = board.clone_and_play(mv);
        match is_double_forced_draw(&child, depth - 1) {
            Some(draw) => draw,
            None => {
                unknown = true;
                true
            }
        }
    });

    if draw_or_unknown && unknown {
        None
    } else {
        Some(draw_or_unknown)
    }
}

pub struct SolverBot<R: Rng> {
    depth: u32,
    rng: R,
}

impl<R: Rng> Debug for SolverBot<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SolverBot {{ depth: {} }}", self.depth)
    }
}

impl<R: Rng> SolverBot<R> {
    pub fn new(depth: u32, rng: R) -> Self {
        assert!(depth > 0);
        SolverBot { depth, rng }
    }
}

impl<B: Board, R: Rng> Bot<B> for SolverBot<R> {
    fn select_move(&mut self, board: &B) -> B::Move {
        minimax(board, &SolverHeuristic, self.depth, &mut self.rng)
            .best_move
            .unwrap()
    }
}
