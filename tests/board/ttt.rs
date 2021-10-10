use crate::board::board_test_main;
use board_game::board::Board;
use board_game::games::ttt::{Coord, TTTBoard};

#[test]
fn empty() {
    board_test_main(&TTTBoard::default())
}

#[test]
fn one_move() {
    let mut board = TTTBoard::default();
    board.play(Coord::new(1, 0));

    board_test_main(&TTTBoard::default())
}

#[test]
fn done() {
    let moves = [(0, 0), (1, 2), (0, 1), (1, 1), (0, 2)];

    let mut board = TTTBoard::default();
    moves.iter().for_each(|&(x, y)| board.play(Coord::new(x, y)));

    board_test_main(&board);
    assert!(board.is_done(), "Board should be done");
}
