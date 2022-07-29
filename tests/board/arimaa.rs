use std::str::FromStr;

use arimaa_engine_step::Action;
use internal_iterator::InternalIterator;

use board_game::board::{Board, BoardMoves, Outcome, Player};
use board_game::games::arimaa::ArimaaBoard;

use crate::board::board_test_main;

#[test]
fn empty() {
    let board = ArimaaBoard::default();
    assert!(!board.is_done());
    board_test_main(&board);
}

#[test]
fn typical_placement() {
    let board = ArimaaBoard::from_str(BASIC_SETUP).unwrap();
    assert!(!board.is_done());
    board_test_main(&board);
}

#[test]
fn can_pass() {
    let mut board = ArimaaBoard::from_str(BASIC_SETUP).unwrap();
    board.play(board.available_moves().next().unwrap());

    assert!(board.is_available_move(Action::Pass));
    board_test_main(&board);
}

#[test]
fn gold_goal() {
    let board = ArimaaBoard::from_str(GOLD_GOAL).unwrap();
    assert_eq!(board.outcome(), Some(Outcome::WonBy(Player::A)));
    board_test_main(&board);
}

const BASIC_SETUP: &str = "
     +-----------------+
    8| r r r r r r r r |
    7| d h c e m c h d |
    6| . . x . . x . . |
    5| . . . . . . . . |
    4| . . . . . . . . |
    3| . . x . . x . . |
    2| D H C M E C H D |
    1| R R R R R R R R |
     +-----------------+
       a b c d e f g h  
";

const GOLD_GOAL: &str = "
    23w
     +-----------------+
    8| r R r r   r r r |
    7|     d           |
    6|   D X c   X     |
    5|         R m     |
    4|                 |
    3|     X     X     |
    2|           d     |
    1| R   R R R R     |
     +-----------------+
       a b c d e f g h
";
