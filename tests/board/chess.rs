use std::collections::hash_map::Entry;
use std::collections::HashMap;

use cozy_chess::{Move, Piece, Square};
use internal_iterator::InternalIterator;
use rand::seq::SliceRandom;
use rand::Rng;

use board_game::board::{Board, BoardMoves};
use board_game::games::chess::{
    move_from_san_str, move_from_uci_str, move_to_san_str, move_to_uci_str, ChessBoard, ParseSanMoveError, Rules,
};
use board_game::util::tiny::consistent_rng;

use crate::board::board_perft_main;

//TODO add tests for 50 move and 3-move rule

#[test]
fn chess_start() {
    chess_board_test_main(&ChessBoard::default());
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

    chess_board_test_main(&board);
}

#[test]
fn test_parse_castle_white() {
    let board = fen_board("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");

    // TODO fix castle encoding, UCI and chess did E1G1 but cozy does E1H1

    // main castle moves
    let short = mv(Square::E1, Square::G1, None);
    let long = mv(Square::E1, Square::C1, None);

    // TODO how is castling encoded?
    assert_eq!(Ok(short), move_from_san_str(&board, "O-O"));
    assert_eq!(Ok(long), move_from_san_str(&board, "O-O-O"));
    assert_eq!(Ok("O-O".to_string()), move_to_san_str(&board, short));
    assert_eq!(Ok("O-O-O".to_string()), move_to_san_str(&board, long));

    // random other checks
    let rook_c1 = mv(Square::A1, Square::C1, None);
    assert_eq!(Ok(rook_c1), move_from_san_str(&board, "Rc1"));
    assert_eq!(Ok("Rc1".to_string()), move_to_san_str(&board, rook_c1));

    // TODO check this again?
    // let short = board.parse_move("O-O").unwrap();
    // assert_eq!(short, board.parse_move("e1h1").unwrap());
    // let long = board.parse_move("O-O-O").unwrap();
    // assert_eq!(long, board.parse_move("e1a1").unwrap());

    chess_board_test_main(&board);
}

#[test]
fn test_parse_castle_black() {
    let board = fen_board("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1");

    // let short = board.parse_move("O-O").unwrap();
    // assert_eq!(short, board.parse_move("e8h8").unwrap());
    // let long = board.parse_move("O-O-O").unwrap();
    // assert_eq!(long, board.parse_move("e8a8").unwrap());

    chess_board_test_main(&board);
}

/// Test cases from <https://www.chessprogramming.org/Perft_Results>.
#[test]
#[ignore] // TODO un-ignore
fn chess_perft() {
    board_perft_main(
        fen_board,
        Some(|b: &ChessBoard| b.inner().to_string()),
        vec![
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                vec![1, 20, 400, 8902, 197281, 4865609],
            ),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                vec![1, 48, 2039, 97862, 4085603],
            ),
        ],
    );
}

#[test]
fn uci_str_invalid() {
    assert!(move_from_uci_str("e2e4").is_ok());
    assert!(move_from_uci_str("e7e8q").is_ok());

    // additional garbage
    assert_eq!(move_from_uci_str("e2e4_garbage").map_err(|_| ()), Err(()));

    // invalid promotions
    assert_eq!(move_from_uci_str("e7e8k").map_err(|_| ()), Err(()));
    assert_eq!(move_from_uci_str("e7e8p").map_err(|_| ()), Err(()));
}

#[test]
fn uci_str() {
    let pairs = vec![
        // basic
        ("e2e4", mv(Square::E2, Square::E4, None)),
        // valid promotions
        ("e7e8q", mv(Square::E7, Square::E8, Some(Piece::Queen))),
        ("e7e8r", mv(Square::E7, Square::E8, Some(Piece::Rook))),
        ("e7e8b", mv(Square::E7, Square::E8, Some(Piece::Bishop))),
        ("e7e8n", mv(Square::E7, Square::E8, Some(Piece::Knight))),
        // basic moves that could mean castling
        ("e1g1", mv(Square::E1, Square::G1, None)),
        ("e1c1", mv(Square::E1, Square::C1, None)),
        ("e8g8", mv(Square::E8, Square::G8, None)),
        ("e8c8", mv(Square::E8, Square::C8, None)),
    ];

    for (s, m) in pairs {
        assert_eq!(s, move_to_uci_str(m));
        assert_eq!(Ok(m), move_from_uci_str(s));
    }
}

// TODO steal test cases from https://github.com/jordanbray/chess/blob/0e538fbee63d01b282cd22a40f1b6655cd6922ab/src/chess_move.rs#L61

// TODO make sure castling maps correctly in kZero!
#[test]
fn derp() {
    println!("{:?}", move_from_san_str(&ChessBoard::default(), "K3e2"));
    println!("{:?}", move_from_san_str(&ChessBoard::default(), "Ke2"));
}

#[test]
fn full_san() {
    let board = fen_board("8/8/8/k6B/6r1/K4B1B/8/8 w - - 0 1");

    let mv = mv(Square::H3, Square::G4, None);

    assert_eq!(Ok(true), board.is_available_move(mv));

    assert_eq!(Ok(mv), move_from_san_str(&board, "Bh3xg4"));
    assert_eq!(
        Err(ParseSanMoveError::MultipleMatchingMoves),
        move_from_san_str(&board, "Bhxg4")
    );
    assert_eq!(
        Err(ParseSanMoveError::MultipleMatchingMoves),
        move_from_san_str(&board, "B3xg4")
    );
    assert_eq!(
        Err(ParseSanMoveError::MultipleMatchingMoves),
        move_from_san_str(&board, "Bxg4")
    );

    assert_eq!(Ok("Bh3xg4"), move_to_san_str(&board, mv).as_ref().map(String::as_str));

    chess_board_test_main(&board);
}

#[test]
fn invalid_san() {
    let board = fen_board("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1");
    let result = move_from_san_str(&board, "h45bx");
    assert!(result.is_err(), "Got {:?}, expected err", result);
}

#[test]
fn fuzz_boards() {
    let mut rng = consistent_rng();

    for i in 0..1000 {
        println!("Start fuzz {}", i);

        let mut board = ChessBoard::default();

        let mut len = 0;

        while board.play_random_available_move(&mut rng).is_ok() {
            test_move_str_loop(&board);
            fuzz_parse_random_san_strings(&board);

            len += 1;
            if len > 1_000 {
                break;
            }
        }
    }
}

fn chess_board_test_main(board: &ChessBoard) {
    // basic tests
    crate::board::board_test_main(board);

    // move string tests
    test_move_str_loop(board);
    fuzz_parse_random_san_strings(board);
}

const MAX_LEN_UCI: usize = 5;
const MAX_LEN_SAN: usize = 6;
const POSSIBLE_CHARS_UCI: &str = "abcdefgh12345678qrbn";
const POSSIBLE_CHARS_SAN: &str = "abcdefgh12345678KQRBN-Ox";

#[test]
fn fuzz_parse_random_uci_strings() {
    let n = 10_000;
    let mut rng = consistent_rng();
    let mut uci = String::new();

    for _ in 0..n {
        uci.clear();
        for _ in 0..rng.gen_range(0..=MAX_LEN_UCI) {
            uci.push(*POSSIBLE_CHARS_UCI.as_bytes().choose(&mut rng).unwrap() as char);
        }

        if let Ok(mv) = move_from_uci_str(&uci) {
            assert_eq!(uci, move_to_uci_str(mv));
        }
    }
}

fn is_subsequence_of(needle: &str, haystack: &str) -> bool {
    let mut haystack = haystack.chars();

    'outer: for n in needle.chars() {
        for h in &mut haystack {
            if n == h {
                continue 'outer;
            }
        }
        if haystack.next().is_none() {
            return false;
        }
    }

    true
}

fn fuzz_parse_random_san_strings(board: &ChessBoard) {
    let n = 10_000;
    let mut rng = consistent_rng();
    let mut san = String::new();

    for _ in 0..n {
        san.clear();
        for _ in 0..rng.gen_range(0..=MAX_LEN_SAN) {
            san.push(*POSSIBLE_CHARS_SAN.as_bytes().choose(&mut rng).unwrap() as char);
        }

        if let Ok(mv) = move_from_san_str(board, &san) {
            assert_eq!(Ok(true), board.is_available_move(mv));

            let minimal_san = move_to_san_str(board, mv).unwrap();
            assert!(
                is_subsequence_of(&minimal_san, &san),
                "{:?} -> {:?} -> {:?} is not a subsequence of orig, board: {:?}",
                san,
                mv,
                minimal_san,
                board
            );
        }
    }
}

fn test_move_str_loop(board: &ChessBoard) {
    if let Ok(moves) = board.available_moves() {
        // move strings
        let mut uci_moves = HashMap::new();
        let mut san_moves = HashMap::new();

        let mut any_duplicate = false;

        moves.for_each(|mv| {
            let uci = move_to_uci_str(mv);
            let san = move_to_san_str(board, mv)
                .unwrap_or_else(|e| panic!("Failed to convert {:?} to SAN, error {:?}", mv, e));

            let missing_uci_char = uci.chars().find(|&c| !POSSIBLE_CHARS_UCI.contains(c));
            assert_eq!(None, missing_uci_char, "{:?}", uci);
            let missing_san_char = san.chars().find(|&c| !POSSIBLE_CHARS_SAN.contains(c));
            assert_eq!(None, missing_san_char, "{:?}", san);

            assert!(uci.len() <= MAX_LEN_UCI, "Found longer UCI: {:?}", uci);
            assert!(san.len() <= MAX_LEN_SAN, "Found longer SAN: {:?}", san);

            let mv_uci = move_from_uci_str(&uci);
            assert_eq!(Ok(mv), mv_uci, "UCI parse failed: {} -> {:?} -> err", mv, uci);
            let mv_san = move_from_san_str(board, &san);
            assert_eq!(Ok(mv), mv_san, "SAN parse failed: {} -> {:?} -> err", mv, san);

            match uci_moves.entry(uci.clone()) {
                Entry::Occupied(entry) => {
                    println!(
                        "Duplicate UCI move string: {} and {} both map to {}",
                        entry.get(),
                        mv,
                        uci
                    );
                    any_duplicate = true;
                }
                Entry::Vacant(entry) => {
                    entry.insert(mv);
                }
            }

            match san_moves.entry(san.clone()) {
                Entry::Occupied(entry) => {
                    println!(
                        "Duplicate SAN move string: {} and {} both map to {}",
                        entry.get(),
                        mv,
                        san
                    );
                    any_duplicate = true;
                }
                Entry::Vacant(entry) => {
                    entry.insert(mv);
                }
            }
        });

        assert!(!any_duplicate, "Found duplicate move string!");
    }
}

fn mv(from: Square, to: Square, promotion: Option<Piece>) -> Move {
    Move { from, to, promotion }
}

fn fen_board(s: &str) -> ChessBoard {
    ChessBoard::from_fen(s, Rules::default()).unwrap()
}
