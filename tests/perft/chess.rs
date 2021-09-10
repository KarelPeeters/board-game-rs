use std::str::FromStr;

use board_game::games::chess::ChessBoard;

use crate::perft::perft_main;

/// Test cases from <https://www.chessprogramming.org/Perft_Results>.
#[test]
fn chess_perft() {
    perft_main(
        |s| ChessBoard::new(chess::Board::from_str(s).unwrap()),
        Some(|b: &ChessBoard| b.inner.to_string()),
        vec![
            ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", vec![1, 20, 400, 8902, 197281, 4865609]),
            ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", vec![1, 48, 2039, 97862, 4085603]),
        ],
    )
}