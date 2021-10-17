use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Neg;

use rand::Rng;

use crate::ai::Bot;
use crate::board::Board;
use crate::util::internal_ext::Control::{Break, Continue};
use crate::util::internal_ext::InternalIteratorExt;

pub trait Heuristic<B: Board>: Debug {
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

    /// Merge old and new into a new value, and compare their values.
    /// For standard minimax searches this can simply be implemented as: `(max(old, new), new.cmp(old))`
    fn merge(old: Self::V, new: Self::V) -> (Self::V, Ordering);
}

#[derive(Debug)]
pub struct MinimaxResult<V, R> {
    /// The value of this board.
    pub value: V,

    /// The result of the [MoveSelector], `None` if the board is done or the depth was zero.
    pub best_move: Option<R>,
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
        RandomMoveSelector::new(rng),
    );

    if result.best_move.is_none() {
        assert!(board.is_done() || depth == 0, "Implementation error in negamax");
    }

    result
}

/// Variant of [minimax] that returns all moves that tie for the best value.
pub fn minimax_all_moves<B: Board, H: Heuristic<B>>(
    board: &B,
    heuristic: &H,
    depth: u32,
) -> MinimaxResult<H::V, Vec<B::Move>> {
    let result = negamax_recurse(
        heuristic,
        board,
        heuristic.value(board, 0),
        0,
        depth,
        None,
        None,
        AllMoveSelector::new(),
    );

    if result.best_move.is_none() {
        assert!(board.is_done() || depth == 0, "Implementation error in negamax");
    }

    result
}

/// Variant of [minimax] that only returns the value and not the best move.
/// The advantage is that no rng is necessary to break ties between best moves.
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

/// The selection procedure for selecting the best move to be returned by [negamax_recurse].
trait MoveSelector<M> {
    type Result;

    fn reset(&mut self);

    fn accept(&mut self, mv: M);

    fn finish(self) -> Self::Result;
}

/// Don't accept any move.
#[derive(Debug)]
struct NoMoveSelector;

impl<M> MoveSelector<M> for NoMoveSelector {
    type Result = ();

    fn reset(&mut self) {}

    fn accept(&mut self, _: M) {}

    fn finish(self) {}
}

/// Accept each move with equal probability,
/// implemented using [reservoir sampling](https://en.wikipedia.org/wiki/Reservoir_sampling).
#[derive(Debug)]
struct RandomMoveSelector<M, R: Rng> {
    picked: Option<M>,
    count: u32,
    rng: R,
}

impl<R: Rng, M> RandomMoveSelector<M, R> {
    pub fn new(rng: R) -> Self {
        RandomMoveSelector {
            picked: None,
            count: 0,
            rng,
        }
    }
}

impl<M, R: Rng> MoveSelector<M> for RandomMoveSelector<M, R> {
    type Result = M;

    fn reset(&mut self) {
        self.count = 0;
        self.picked = None;
    }

    fn accept(&mut self, mv: M) {
        self.count += 1;
        if self.rng.gen_range(0..self.count) == 0 {
            self.picked = Some(mv)
        }
    }

    fn finish(self) -> Self::Result {
        self.picked.expect("we should have selected a move by now")
    }
}

#[derive(Debug)]
struct AllMoveSelector<M> {
    moves: Vec<M>,
}

impl<M> AllMoveSelector<M> {
    pub fn new() -> Self {
        AllMoveSelector {
            moves: Default::default(),
        }
    }
}

impl<M> MoveSelector<M> for AllMoveSelector<M> {
    type Result = Vec<M>;

    fn reset(&mut self) {
        self.moves.clear();
    }

    fn accept(&mut self, mv: M) {
        self.moves.push(mv)
    }

    fn finish(self) -> Self::Result {
        self.moves
    }
}

/// The core minimax implementation.
/// Alpha-Beta Negamax, implementation based on
/// <https://en.wikipedia.org/wiki/Negamax#Negamax_with_alpha_beta_pruning>
fn negamax_recurse<B: Board, H: Heuristic<B>, S: MoveSelector<B::Move>>(
    heuristic: &H,
    board: &B,
    board_heuristic: H::V,
    length: u32,
    depth_left: u32,
    alpha: Option<H::V>,
    beta: Option<H::V>,
    mut move_selector: S,
) -> MinimaxResult<H::V, S::Result> {
    if depth_left == 0 || board.is_done() {
        return MinimaxResult {
            value: board_heuristic,
            best_move: None,
        };
    }

    let mut best_value = None;
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

        let (new_best_value, ordering) = best_value.map_or((child_value, Ordering::Greater), |best_value| {
            H::merge(best_value, child_value)
        });
        let new_alpha = alpha.map_or(new_best_value, |alpha| H::merge(alpha, new_best_value).0);

        best_value = Some(new_best_value);

        if ordering.is_gt() {
            move_selector.reset();
        }
        if ordering.is_ge() {
            move_selector.accept(mv);
        }
        alpha = Some(new_alpha);

        if beta.map_or(false, |beta| H::merge(beta, new_alpha).1.is_ge()) {
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
            best_move: Some(move_selector.finish()),
        }
    }
}

pub struct MiniMaxBot<B: Board, H: Heuristic<B>, R: Rng> {
    depth: u32,
    heuristic: H,
    rng: R,
    ph: PhantomData<B>,
}

impl<B: Board, H: Heuristic<B>, R: Rng> Debug for MiniMaxBot<B, H, R> {
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
