use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Neg;

use rand::Rng;

use crate::ai::Bot;
use crate::board::Board;
use crate::util::internal_ext::Control::{Break, Continue};
use crate::util::internal_ext::InternalIteratorExt;

pub trait Heuristic<B: Board> {
    /// The type used to represent the heuristic value of a board.
    type V: Copy + Neg<Output = Self::V>;

    /// Return the heuristic value for the given board from the the next player POV.
    /// `depth` is the current depth, the number of moves played since the board the search was started on.
    /// Can be used to prefer faster wins or slower losses.
    fn value(&self, board: &B, depth: u32) -> Self::V;

    /// Return the value of `child`, given the previous board, its value and the move that was just played.
    /// This function can be overridden to improve performance.
    ///
    /// Given:
    /// * `child = board.clone_and_play(mv)`
    /// * `board_value = value(board, board_length)`
    /// * `child_length = board_length + 1`
    ///
    /// This function must ensure that
    /// * `value(child, child_length) == value_update(board, board_value, board_length, mv, child)`
    #[allow(unused_variables)]
    fn value_update(&self, board: &B, board_value: Self::V, board_length: u32, mv: B::Move, child: &B) -> Self::V {
        self.value(child, board_length + 1)
    }

    /// Merge old and new into a new value, and return whether the new value is at least as good the old one.
    /// For standard minimax searches this can simply be implemented as: `(max(old, new), new >= old)`
    fn merge(old: Self::V, new: Self::V) -> (Self::V, bool);
}

#[derive(Debug)]
pub struct MinimaxResult<V, M> {
    /// The value of this board.
    pub value: V,

    /// The best move to play, `None` is the board is done or the search depth was 0.
    pub best_move: Option<M>,
}

/// Evaluate the board using minimax with the given heuristic up to the given depth.
/// Return both the value and the best move. If multiple moves have the same value pick a random one using `rng`.
/// The returned value is from the POV of `board.next_player`.
pub fn minimax<B: Board, H: Heuristic<B>>(
    board: &B,
    heuristic: &H,
    depth: u32,
    rng: &mut impl Rng,
) -> MinimaxResult<H::V, B::Move> {
    let result = negamax_recurse(
        heuristic,
        board,
        heuristic.value(board, 0),
        0,
        depth,
        None,
        None,
        RandomBestMoveSelector::new(rng),
    );

    if result.best_move.is_none() {
        assert!(board.is_done() || depth == 0, "Implementation error in negamax");
    }

    result
}

/// Evaluate the board using minimax with the given heuristic up to the given depth.
/// Only returns the value without selecting a move, and so doesn't require an `Rng`.
pub fn minimax_value<B: Board, H: Heuristic<B>>(board: &B, heuristic: &H, depth: u32) -> H::V {
    negamax_recurse(
        heuristic,
        board,
        heuristic.value(board, 0),
        0,
        depth,
        None,
        None,
        NoMoveSelector,
    )
    .value
}

/// This is a trait so negamax_recurse is instantiated twice,
/// once for the top-level search with move selection and once for deeper nodes without any moves.
trait MoveSelector {
    fn accept(&mut self) -> bool;
}

/// Don't accept any move.
struct NoMoveSelector;

impl MoveSelector for NoMoveSelector {
    fn accept(&mut self) -> bool {
        false
    }
}

/// Implement each move with equal probability,
/// implemented using [reservoir sampling](https://en.wikipedia.org/wiki/Reservoir_sampling).
struct RandomBestMoveSelector<'a, R: Rng> {
    rng: &'a mut R,
    count: u32,
}

impl<'a, R: Rng> RandomBestMoveSelector<'a, R> {
    pub fn new(rng: &'a mut R) -> Self {
        RandomBestMoveSelector { rng, count: 0 }
    }
}

impl<R: Rng> MoveSelector for RandomBestMoveSelector<'_, R> {
    fn accept(&mut self) -> bool {
        self.count += 1;
        self.rng.gen_range(0..self.count) == 0
    }
}

/// The core minimax implementation.
/// Alpha-Beta Negamax, implementation based on
/// <https://en.wikipedia.org/wiki/Negamax#Negamax_with_alpha_beta_pruning>
fn negamax_recurse<B: Board, H: Heuristic<B>>(
    heuristic: &H,
    board: &B,
    board_heuristic: H::V,
    length: u32,
    depth_left: u32,
    alpha: Option<H::V>,
    beta: Option<H::V>,
    mut move_selector: impl MoveSelector,
) -> MinimaxResult<H::V, B::Move> {
    if depth_left == 0 || board.is_done() {
        return MinimaxResult {
            value: board_heuristic,
            best_move: None,
        };
    }

    let mut best_value = None;
    let mut best_move: Option<B::Move> = None;

    let mut alpha = alpha;

    let early = board.available_moves().for_each_control(|mv: B::Move| {
        let child = board.clone_and_play(mv);
        let child_heuristic = heuristic.value_update(board, board_heuristic, length, mv, &child);

        let child_value = -negamax_recurse(
            heuristic,
            &child,
            child_heuristic,
            length + 1,
            depth_left - 1,
            beta.map(Neg::neg),
            alpha.map(Neg::neg),
            NoMoveSelector,
        )
        .value;

        let (new_best_value, is_gte) =
            best_value.map_or((child_value, true), |best_value| H::merge(best_value, child_value));
        let new_alpha = alpha.map_or(new_best_value, |alpha| H::merge(alpha, new_best_value).0);

        best_value = Some(new_best_value);
        if is_gte && move_selector.accept() {
            best_move = Some(mv);
        }
        alpha = Some(new_alpha);

        if beta.map_or(false, |beta| H::merge(beta, new_alpha).1) {
            Break(MinimaxResult {
                value: new_best_value,
                best_move: None,
            })
        } else {
            Continue
        }
    });

    if let Some(early) = early {
        early
    } else {
        MinimaxResult {
            value: best_value.unwrap(),
            best_move,
        }
    }
}

pub struct MiniMaxBot<B: Board, H: Heuristic<B>, R: Rng> {
    depth: u32,
    heuristic: H,
    rng: R,
    ph: PhantomData<B>,
}

impl<B: Board, H: Heuristic<B> + Debug, R: Rng> Debug for MiniMaxBot<B, H, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MiniMaxBot {{ depth: {}, heuristic: {:?} }}",
            self.depth, self.heuristic
        )
    }
}

impl<B: Board, H: Heuristic<B>, R: Rng> MiniMaxBot<B, H, R> {
    pub fn new(depth: u32, heuristic: H, rng: R) -> Self {
        assert!(depth > 0, "requires depth>0 to find the best move");
        MiniMaxBot {
            depth,
            heuristic,
            rng,
            ph: PhantomData,
        }
    }
}

impl<B: Board, H: Heuristic<B> + Debug, R: Rng> Bot<B> for MiniMaxBot<B, H, R> {
    fn select_move(&mut self, board: &B) -> B::Move {
        assert!(!board.is_done());
        // SAFETY: unwrap is safe because:
        // * depth > 0 (see [`MiniMaxBot::new`])
        // * the board is not done (see assert)
        // * the assert in [`minimax`] states that
        //     best_move.is_none() => board.is_done() || depth == 0
        //   by contraposition, we have
        //     !board.is_done() && depth > 0 => best_move.is_some()
        // hence best_move.is_some()
        minimax(board, &self.heuristic, self.depth, &mut self.rng)
            .best_move
            .unwrap()
    }
}
