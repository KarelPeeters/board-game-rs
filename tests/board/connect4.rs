use crate::board::board_test_main;
use board_game::board::Outcome::WonBy;
use board_game::board::{Board, Outcome, Player};
use board_game::games::connect4::Connect4;
use board_game::util::board_gen::board_with_moves;

#[test]
fn empty() {
    board_test_main(&Connect4::default());
}

#[test]
fn basic() {
    board_test_main(&board_with_moves(Connect4::default(), &[1]));
    board_test_main(&board_with_moves(Connect4::default(), &[1, 2]));
    board_test_main(&board_with_moves(Connect4::default(), &[1, 2, 3]));
}

#[test]
fn test_lines() {
    check_outcome(&[1, 1, 2, 2, 3, 3, 4], Some(WonBy(Player::A)));
    check_outcome(&[1, 2, 1, 2, 1, 2, 1], Some(WonBy(Player::A)));
    check_outcome(&[1, 2, 2, 3, 6, 3, 3, 4, 6, 4, 6, 4, 4], Some(WonBy(Player::A)));
    check_outcome(&[4, 3, 3, 2, 6, 2, 2, 1, 6, 1, 6, 1, 1], Some(WonBy(Player::A)));
}

fn check_outcome(moves: &[u8], outcome: Option<Outcome>) {
    let board = board_with_moves(Connect4::default(), moves);
    println!("moves: {:?}", moves);
    println!("{}", board);

    assert_eq!(board.outcome(), outcome);

    board_test_main(&board);
}
