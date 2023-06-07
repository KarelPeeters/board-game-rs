//! Utilities to generate a `Board` in a random state.
use rand::Rng;

pub use crate::ai::solver::is_double_forced_draw;
use crate::ai::solver::solve_value;
use crate::board::{Board, BoardDone, Outcome, Player};
use crate::pov::{NonPov, Pov};
use crate::wdl::OutcomeWDL;

/// Play the given moves, starting from `start`.
pub fn board_with_moves<B: Board>(start: B, moves: &[B::Move]) -> B {
    let mut curr = start;
    for &mv in moves {
        assert!(!curr.is_done(), "Board already done, playing {} on {}", mv, curr);
        assert_eq!(
            curr.is_available_move(mv),
            Ok(true),
            "Move not available, playing {} on {}",
            mv,
            curr
        );
        curr.play(mv).unwrap();
    }
    curr
}

/// Generate a `Board` by playing `n` random moves on `start`.
pub fn random_board_with_moves<B: Board>(start: &B, n: u32, rng: &mut impl Rng) -> B {
    //this implementation could be made faster with backtracking instead of starting from scratch,
    // but this only starts to matter for very high n and that's not really the main use case
    'new_try: loop {
        let mut board = start.clone();
        for _ in 0..n {
            match board.play_random_available_move(rng) {
                Ok(()) => {}
                Err(BoardDone) => continue 'new_try,
            }
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
            board.play_random_available_move(rng).unwrap();
        }
    }
}

/// Generate a `Board` by playing random moves until a forced win in `depth` moves is found
/// for `board.next_player`, which may be different from `start.next_player`.
pub fn random_board_with_forced_win<B: Board>(start: &B, depth: u32, rng: &mut impl Rng) -> B {
    // TODO this is no longer true for non-alternating boards, maybe add a boolean getter for it back?
    // if !B::can_lose_after_move() {
    //     assert_eq!(
    //         depth % 2,
    //         1,
    //         "forced win in an even number of moves is impossible (because the last move would be by the opponent)"
    //     );
    // }

    random_board_with_depth_condition(start, depth, rng, |board, depth| {
        solve_value(board, depth).to_outcome_wdl() == Some(OutcomeWDL::Win)
    })
}

/// Generate a `Board` by playing random moves until a forced win in `depth` moves is found for `player`.
pub fn random_board_with_forced_win_for<B: Board>(start: &B, depth: u32, player: Player, rng: &mut impl Rng) -> B {
    random_board_with_depth_condition(start, depth, rng, |board, depth| {
        solve_value(board, depth)
            .to_outcome_wdl()
            .un_pov(board.next_player())
            .pov(player)
            == Some(OutcomeWDL::Win)
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
    assert!(
        !start.is_done(),
        "Start board is done and does not match condition, so we won't find anything that does"
    );

    loop {
        let mut board = start.clone();
        while let Ok(()) = board.play_random_available_move(rng) {
            if cond(&board) {
                return board;
            }
        }
    }
}

/// Generate a random board such that `cond(board, depth) & !cond(board, depth-1)`.
pub fn random_board_with_depth_condition<B: Board>(
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

            match board.play_random_available_move(rng) {
                Ok(()) => {}
                Err(BoardDone) => break,
            }
        }
    }
}

/// Iterator over randomly generated boards.
/// Yields all intermediate boards, including the start and end of each simulation.
#[derive(Debug, Clone)]
pub struct RandomBoardIterator<B: Board, R: Rng> {
    start: B,
    rng: R,
    curr: B,
}

impl<B: Board, R: Rng> RandomBoardIterator<B, R> {
    pub fn new(start: B, rng: R) -> Result<Self, BoardDone> {
        start.check_done()?;
        Ok(RandomBoardIterator {
            start: start.clone(),
            rng,
            curr: start,
        })
    }
}

impl<B: Board, R: Rng> Iterator for RandomBoardIterator<B, R> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.curr.clone();

        match self.curr.play_random_available_move(&mut self.rng) {
            Ok(()) => {}
            Err(BoardDone) => {
                self.curr = self.start.clone();
            }
        }

        Some(result)
    }
}
