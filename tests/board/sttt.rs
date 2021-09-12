use board_game::board::{Board, Outcome};
use board_game::games::sttt::{board_from_compact_string, STTTBoard};

use crate::board::board_test_main;

#[test]
fn sttt_empty() {
    board_test_main(&STTTBoard::default())
}

#[test]
fn sttt_few() {
    let board =
        board_from_compact_string("                        o  .........               o    x  xxox        x   O  o  ");
    board_test_main(&board)
}

#[test]
fn sttt_close() {
    let board =
        board_from_compact_string("x     ooo.Ooo.xx..o  o  oxoxxxoo     x  xo oxx  xo o  x xxooxx  oxox oox  xx xoxo");
    board_test_main(&board)
}

#[test]
fn sttt_draw() {
    let board =
        board_from_compact_string("xxx o xo ooXoxxxxoo  o  oxo o xxx   oxoxxoxoxo xooo x oxxoxxoo  xxooxo xxoooxxoxo");
    assert_eq!(board.outcome(), Some(Outcome::Draw));
    board_test_main(&board)
}
