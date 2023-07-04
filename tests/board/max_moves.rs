use std::str::FromStr;

use internal_iterator::InternalIterator;

use board_game::board::{Board, BoardMoves, Outcome, Player};
use board_game::games::dummy::DummyGame;
use board_game::games::max_length::MaxMovesBoard;

#[test]
fn basic_draw() {
    let dummy = DummyGame::from_str("((((A))))").unwrap();
    let board = MaxMovesBoard::new(dummy, 2);

    test_outcomes(board, &[None, None, Some(Outcome::Draw)]);
}

#[test]
fn basic_finished() {
    let dummy = DummyGame::from_str("((((A))))").unwrap();
    let board = MaxMovesBoard::new(dummy, 10);

    test_outcomes(board, &[None, None, None, None, Some(Outcome::WonBy(Player::A))]);
}

fn test_outcomes(mut board: MaxMovesBoard<DummyGame>, outcomes: &[Option<Outcome>]) {
    for &outcome in outcomes {
        println!("{}", board);

        assert_eq!(outcome, board.outcome());
        if outcome.is_some() {
            break;
        }

        let mv = board.available_moves().unwrap().next().unwrap();
        board.play(mv).unwrap();
    }
}
