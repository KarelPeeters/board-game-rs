use internal_iterator::InternalIterator;

use board_game::ai::solver::{solve_all_moves, solve_value, SolverValue};
use board_game::board::{Board, BoardMoves};
use board_game::games::ttt::{Coord, TTTBoard};
use board_game::util::game_stats::all_possible_boards;
use board_game::wdl::OutcomeWDL;

#[test]
fn solver_ttt_root() {
    let board = TTTBoard::default;
    let root_eval = solve_value(&board(), 20);
    assert_eq!(
        root_eval.to_outcome_wdl(),
        Some(OutcomeWDL::Draw),
        "TTT is a theoretical draw, eval {:?}",
        root_eval
    );
}

#[test]
fn solver_ttt_win() {
    let mut board = TTTBoard::default();
    board.play(Coord::from_xy(0, 0));
    board.play(Coord::from_xy(1, 0));
    println!("{}", board);

    let root_eval = solve_all_moves(&board, 20);
    println!("{:?}", root_eval);

    assert_eq!(Some(OutcomeWDL::Win), root_eval.value.to_outcome_wdl());
}

#[test]
fn solver_ttt_loss() {
    let mut board = TTTBoard::default();
    board.play(Coord::from_xy(0, 0));
    board.play(Coord::from_xy(1, 0));
    board.play(Coord::from_xy(1, 1));
    println!("{}", board);

    let root_eval = solve_all_moves(&board, 20);
    println!("{:?}", root_eval);

    assert_eq!(Some(OutcomeWDL::Loss), root_eval.value.to_outcome_wdl());
}

#[test]
fn solver_ttt_consistent() {
    let boards = all_possible_boards(&TTTBoard::default(), 20, false);

    for board in boards {
        let eval = solve_all_moves(&board, 20);

        println!("{}", board);
        println!("{:?}", eval);

        board.available_moves().for_each(|mv| {
            let child = board.clone_and_play(mv);

            let child_eval = solve_value(&child, 20);
            println!("  {:?}: {:?}", mv, child_eval);

            assert!(
                SolverValue::could_be_optimal_child(eval.value, child_eval),
                "child {:?} cannot be better then parent {:?}",
                child_eval,
                eval.value,
            )
        });

        println!();
        println!();
    }
}
