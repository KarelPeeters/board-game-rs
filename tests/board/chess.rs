use board_game::board::Board;
use board_game::games::chess::{ChessBoard, Rules};

use crate::board::{board_perft_main, board_test_main};

//TODO add tests for 50 move and 3-move rule

#[test]
fn chess_start() {
    board_test_main(&ChessBoard::default());
}

#[test]
fn chess_en_passant() {
    let moves = vec!["e4", "e6", "e5", "d5"];

    let mut board = ChessBoard::default();
    for &mv in &moves {
        println!("{}", board);
        board.play(board.parse_move(mv).unwrap()).unwrap();
    }

    let capture = board.parse_move("ed6").unwrap();
    assert!(board.is_available_move(capture).unwrap());

    board_test_main(&board);
}

#[test]
fn test_parse_castle_white() {
    let board =
        ChessBoard::new_without_history_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", Rules::default())
            .unwrap();

    let short = board.parse_move("O-O").unwrap();
    assert_eq!(short, board.parse_move("e1h1").unwrap());
    let long = board.parse_move("O-O-O").unwrap();
    assert_eq!(long, board.parse_move("e1a1").unwrap());
}

#[test]
fn test_parse_castle_black() {
    let board =
        ChessBoard::new_without_history_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1", Rules::default())
            .unwrap();

    let short = board.parse_move("O-O").unwrap();
    assert_eq!(short, board.parse_move("e8h8").unwrap());
    let long = board.parse_move("O-O-O").unwrap();
    assert_eq!(long, board.parse_move("e8a8").unwrap());
}

/// Test cases from <https://www.chessprogramming.org/Perft_Results>.
#[test]
fn chess_perft() {
    #[rustfmt::skip]
        board_perft_main(
        |s| ChessBoard::new_without_history_fen(s, Rules::default()).unwrap(),
        Some(|b: &ChessBoard| b.inner().to_string()),
        vec![
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", vec![1, 20, 400, 8902, 197281, 4865609]),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", vec![1, 48, 2039, 97862, 4085603]),
        ],
    );
}
