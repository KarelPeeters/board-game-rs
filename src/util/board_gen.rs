//! Utilities to generate a `Board` in a random state.
use rand::Rng;

pub use crate::ai::solver::is_double_forced_draw;
use crate::ai::solver::solve_value;
use crate::board::{Board, Outcome};
use crate::wdl::OutcomeWDL;

/// Play the given moves, starting from `start`.
pub fn board_with_moves<B: Board>(start: B, moves: &[B::Move]) -> B {
    let mut curr = start;
    for &mv in moves {
        assert!(!curr.is_done(), "Board already done, playing {} on {}", mv, curr);
        assert!(
            curr.is_available_move(mv),
            "Move not available, playing {} on {}",
            mv,
            curr
        );
        curr.play(mv);
    }
    curr
}

/// Generate a `Board` by playing `n` random moves on `start`.
pub fn random_board_with_moves<B: Board>(start: &B, n: u32, rng: &mut impl Rng) -> B {
    //this implementation could be made faster with backtracking instead of starting from scratch,
    // but this only starts to matter for very high n and that's not really the main use case

    'newtry: loop {
        let mut board = start.clone();
        for _ in 0..n {
            if board.is_done() {
                continue 'newtry;
            }
            board.play(board.random_available_move(rng))
        }
        return board;
    }
}

/// Generate a random `Board` with a specific `Outcome`.
pub fn random_board_with_outcome<B: Board>(start: &B, outcome: Outcome, rng: &mut impl Rng) -> B {
    loop {
        let mut board = start.clone();
        loop {
            if let Some(actual) = board.outcome() {
                if actual == outcome {
                    return board;
                }
                break;
            }

            board.play(board.random_available_move(rng))
        }
    }
}

/// Generate a `Board` by playing random moves until a forced win in `depth` moves is found
/// for `board.next_player`, which may be different from `start.next_player`.
pub fn random_board_with_forced_win<B: Board>(start: &B, depth: u32, rng: &mut impl Rng) -> B {
    if !B::can_lose_after_move() {
        assert_eq!(
            depth % 2,
            1,
            "forced win in an even number of moves is impossible (because the last move would be by the opponent)"
        );
    }

    random_board_with_depth_condition(start, depth, rng, |board, depth| {
        solve_value(board, depth).to_outcome_wdl() == Some(OutcomeWDL::Win)
    })
}

/// Generate a random board with a *double forced draw* in `depth` moves, meaning that no matter what either player does
/// it's impossible for someone to win.
pub fn random_board_with_double_forced_draw<B: Board>(start: &B, depth: u32, rng: &mut impl Rng) -> B {
    random_board_with_depth_condition(start, depth, rng, |board, depth| {
        is_double_forced_draw(board, depth).unwrap_or(false)
    })
}

/// Generate a `Board` by playing random moves until `cond(&board)` returns true.
pub fn random_board_with_condition<B: Board>(start: &B, rng: &mut impl Rng, mut cond: impl FnMut(&B) -> bool) -> B {
    if cond(start) {
        return start.clone();
    }

    loop {
        let mut board = start.clone();

        while !board.is_done() {
            board.play(board.random_available_move(rng));

            if cond(&board) {
                return board;
            }
        }
    }
}

/// Generate a random board such that `cond(board, depth) & !cond(board, depth-1)`.
fn random_board_with_depth_condition<B: Board>(
    start: &B,
    depth: u32,
    rng: &mut impl Rng,
    cond: impl Fn(&B, u32) -> bool,
) -> B {
    loop {
        let mut board = start.clone();

        loop {
            let deep_match = cond(&board, depth);
            if deep_match {
                let shallow_match = depth > 0 && cond(&board, depth - 1);
                if shallow_match {
                    break;
                }

                return board;
            }

            if board.is_done() {
                break;
            }
            board.play(board.random_available_move(rng));
        }
    }
}
