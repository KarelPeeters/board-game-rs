use board_game::board::Board;
use board_game::games::oware::OwareBoard;
use board_game::util::board_gen::board_with_moves;

use crate::board::board_test_main;

#[test]
fn empty() {
    board_test_main(&OwareBoard::<6>::default());
}

#[test]
fn one_move() {
    let mut board = OwareBoard::<6>::default();
    board.play(3).unwrap();

    board_test_main(&OwareBoard::<6>::default())
}

#[test]
fn done() {
    // One of the Shortest possible game
    let moves = [5, 2, 4, 1, 2, 5, 3, 1, 1, 3, 0, 4];

    let board = board_with_moves(OwareBoard::<6>::default(), &moves);
    board_test_main(&board);
    assert!(board.is_done(), "Board should be done");
}
