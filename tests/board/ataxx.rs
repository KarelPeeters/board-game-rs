use internal_iterator::InternalIterator;

use board_game::board::{Board, BoardAvailableMoves, Outcome, Player};
use board_game::games::ataxx::{AtaxxBoard, Move};

use crate::board::board_test_main;

#[test]
fn ataxx_empty() {
    board_test_main(&AtaxxBoard::default())
}

#[test]
fn ataxx_few() {
    board_test_main(&AtaxxBoard::from_fen("2x3o/1x3oo/7/7/7/7/o3x2 o 0 1"));
}

#[test]
fn ataxx_close() {
    let board = AtaxxBoard::from_fen("ooooooo/xxxxooo/oxxxoo1/oxxxooo/ooxoooo/xxxxxoo/xxxxxxx x 0 1");
    board_test_main(&board)
}

#[test]
fn ataxx_done_clear() {
    let board = AtaxxBoard::from_fen("4x2/4xx1/xxx4/1x5/4x2/7/7 o 2 1");
    assert_eq!(Some(Outcome::WonBy(Player::A)), board.outcome());
    board_test_main(&board)
}

#[test]
fn ataxx_done_full() {
    let board = AtaxxBoard::from_fen("xxxoxxx/ooooxxx/ooooxxx/xxxooox/xxxooox/xxxxxxx/ooooxxx o 0 1");
    assert_eq!(Some(Outcome::WonBy(Player::A)), board.outcome());
    board_test_main(&board)
}

#[test]
fn ataxx_forced_pass() {
    let board = AtaxxBoard::from_fen("xxxxxxx/-------/-------/o6/7/7/7 x 0 0");
    assert!(!board.is_done(), "Board is not done, player B can still play");
    assert!(board.available_moves().all(|mv| mv == Move::Pass));
    board_test_main(&board)
}
