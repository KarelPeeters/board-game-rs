use internal_iterator::InternalIterator;
use rand::rngs::SmallRng;
use rand::SeedableRng;

use board_game::ai::solver::{solve, solve_value, SolverValue};
use board_game::board::{Board, BoardAvailableMoves};
use board_game::games::ttt::TTTBoard;
use board_game::util::game_stats::all_possible_boards;
use board_game::wdl::OutcomeWDL;

#[test]
fn solver_ttt_root() {
    let root_eval = solve_value(&TTTBoard::default(), 20);
    assert!(
        root_eval.to_outcome_wdl() == Some(OutcomeWDL::Draw),
        "TTT is a theoretical draw, eval {:?}",
        root_eval,
    );
}

#[test]
fn solver_ttt_consistent() {
    let boards = all_possible_boards(&TTTBoard::default(), false);
    let mut rng = SmallRng::seed_from_u64(0);

    for board in boards {
        let eval = solve(&board, 20, &mut rng);

        println!("{:?}", eval);
        println!("{}", board);

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
