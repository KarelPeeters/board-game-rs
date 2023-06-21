use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::ops::Neg;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::ai::minimax::{minimax, minimax_all_moves, minimax_last_move, minimax_value, Heuristic, MinimaxResult};
use crate::ai::Bot;
use crate::board::{Board, BoardDone, Outcome};
use crate::pov::NonPov;
use crate::wdl::OutcomeWDL;

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

    fn merge(old: SolverValue, new: SolverValue) -> (SolverValue, Ordering) {
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

    /// Return `(best_value, cmp(new, old))`,
    /// where best_value properly accounts for shortest win, longest loss and draw vs unknown.
    pub fn merge(old: SolverValue, new: SolverValue) -> (SolverValue, Ordering) {
        use SolverValue::*;

        match (old, new) {
            // prefer shortest win and longest loss
            (WinIn(old_n), WinIn(new_n)) => (if new_n <= old_n { new } else { old }, new_n.cmp(&old_n).reverse()),
            (LossIn(old_n), LossIn(new_n)) => (if new_n >= old_n { new } else { old }, new_n.cmp(&old_n)),

            // win/loss is better/worse then everything else
            (WinIn(_), _) => (old, Ordering::Less),
            (LossIn(_), _) => (new, Ordering::Greater),
            (_, WinIn(_)) => (new, Ordering::Greater),
            (_, LossIn(_)) => (old, Ordering::Less),

            // draw and unknown are the same, but return unknown if either is unknown
            (Draw, Draw) => (Draw, Ordering::Equal),
            (Unknown | Draw, Unknown | Draw) => (Unknown, Ordering::Equal),
        }
    }

    /// Return whether `child` could a child of the optimally combined `parent`.
    pub fn could_be_optimal_child(parent: SolverValue, child: SolverValue) -> bool {
        let best_child = match parent {
            SolverValue::WinIn(n) => SolverValue::LossIn(n - 1),
            SolverValue::LossIn(n) => SolverValue::WinIn(n - 1),
            SolverValue::Draw => SolverValue::Draw,
            SolverValue::Unknown => panic!("This function does not work for unknown values"),
        };

        // evaluate child >= best_child since this is from the other POV
        SolverValue::merge(best_child, child).1.is_ge()
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

pub fn solve_all_moves<B: Board>(board: &B, depth: u32) -> MinimaxResult<SolverValue, Vec<B::Move>> {
    minimax_all_moves(board, &SolverHeuristic, depth)
}

pub fn solve_last_move<B: Board>(board: &B, depth: u32) -> MinimaxResult<SolverValue, B::Move> {
    minimax_last_move(board, &SolverHeuristic, depth)
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
    let draw_or_unknown = board
        .children()
        .unwrap()
        .all(|(_, child)| match is_double_forced_draw(&child, depth - 1) {
            Some(draw) => draw,
            None => {
                unknown = true;
                true
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
    fn select_move(&mut self, board: &B) -> Result<B::Move, BoardDone> {
        board.check_done()?;
        Ok(minimax(board, &SolverHeuristic, self.depth, &mut self.rng)
            .best_move
            .unwrap())
    }
}
