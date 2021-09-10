use board_game::board::{Board, Outcome, Player};
use board_game::games::ataxx::AtaxxBoard;
use board_game::util::board_gen::{random_board_with_forced_win, random_board_with_moves, random_board_with_outcome};

use crate::board::{board_test_main, consistent_rng};

#[test]
fn ataxx_empty() {
    board_test_main(&AtaxxBoard::default())
}

#[test]
fn ataxx_few() {
    board_test_main(&random_board_with_moves(&AtaxxBoard::default(), 10, &mut consistent_rng()))
}

#[test]
fn ataxx_close() {
    let mut rng = consistent_rng();

    // generate a board that's pretty full instead of the more likely empty board
    let start = random_board_with_moves(&AtaxxBoard::default(), 120, &mut rng);
    let board = random_board_with_forced_win(&start, 5, &mut rng);

    board_test_main(&board)
}

#[test]
fn ataxx_done() {
    board_test_main(&random_board_with_outcome(&AtaxxBoard::default(), Outcome::WonBy(Player::A), &mut consistent_rng()))
}

#[test]
fn ataxx_forced_pass() {
    let board = AtaxxBoard::from_fen("xxxxxxx/-------/-------/o6/7/7/7 x 0 0");
    assert!(!board.is_done(), "Board is not done, player B can still play");
    board_test_main(&board)
}