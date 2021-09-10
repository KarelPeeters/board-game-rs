use board_game::board::Outcome;
use board_game::games::sttt::STTTBoard;
use board_game::util::board_gen::{random_board_with_forced_win, random_board_with_moves, random_board_with_outcome};

use crate::board::{board_test_main, consistent_rng};

#[test]
fn sttt_empty() {
    board_test_main(&STTTBoard::default())
}

#[test]
fn sttt_few() {
    board_test_main(&random_board_with_moves(&STTTBoard::default(), 10, &mut consistent_rng()))
}

#[test]
fn sttt_close() {
    board_test_main(&random_board_with_forced_win(&STTTBoard::default(), 5, &mut consistent_rng()))
}

#[test]
fn sttt_draw() {
    board_test_main(&random_board_with_outcome(&STTTBoard::default(), Outcome::Draw, &mut consistent_rng()))
}