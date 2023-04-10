use board_game::board::{Board, Outcome};
use board_game::games::go::{GoBoard, Move, Rules, Tile};
use board_game::util::board_gen::board_with_moves;
use board_game::util::game_stats::{perf_naive, perft};

use crate::board::{board_test_main, print_board_with_moves};

#[test]
fn empty_0() {
    let cases = [
        (0, "/ b 0"),
        (1, ". b 0"),
        (2, "../.. b 0"),
        (5, "...../...../...../...../..... b 0"),
        (19, ".................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../.................../................... b 0"),
    ];

    let rules = Rules::tromp_taylor();

    for (size, fen) in cases {
        let board = GoBoard::new(size, rules);
        assert_eq!(board.to_fen(), fen);
        assert_eq!(Ok(&board), GoBoard::from_fen(fen, rules).as_ref());

        board_test_main(&board);
    }
}

#[test]
fn parse_fen() {
    let tiles = [(3, 3), (4, 3), (3, 2), (0, 1), (0, 4), (4, 4), (1, 0)];

    let rules = Rules::tromp_taylor();
    let board = board_with_moves(
        GoBoard::new(5, rules),
        &tiles.map(|(x, y)| Move::Place(Tile::new(x, y))),
    );

    assert_eq!("b...w/...bw/...b./w..../.b... w 0", board.to_fen());

    let board_white = board.clone_and_play(Move::Place(Tile::new(0, 0))).unwrap();
    assert_eq!("b...w/...bw/...b./w..../wb... b 0", board_white.to_fen());

    let board_pass = board.clone_and_play(Move::Pass).unwrap();
    assert_eq!("b...w/...bw/...b./w..../.b... b 1", board_pass.to_fen());

    let board_done = board_pass.clone_and_play(Move::Pass).unwrap();
    assert_eq!("b...w/...bw/...b./w..../.b... w 2", board_done.to_fen());

    for board in [board, board_white, board_pass, board_done] {
        println!("Checking loopback for\n{}", board);
        let parsed = GoBoard::from_fen(&board.to_fen(), rules);

        if let Ok(parsed) = &parsed {
            println!("Parsed:\n{}", parsed);
        }

        assert_eq!(parsed, Ok(board));
    }
}

#[test]
fn clear_corner() {
    let rules = Rules::tromp_taylor();
    let start = GoBoard::new(5, rules);
    let moves = [(0, 0), (0, 1), (4, 4), (1, 0)].map(|(x, y)| Move::Place(Tile::new(x, y)));

    let board = print_board_with_moves(start, &moves);
    assert_eq!(board.tile(Tile::new(0, 0)), None);

    board_test_main(&board);
}

#[test]
fn double_pass() {
    let rules = Rules::tromp_taylor();
    let start = GoBoard::new(5, rules);
    let moves = [Move::Pass, Move::Pass];

    let board = print_board_with_moves(start, &moves);
    assert_eq!(board.outcome(), Some(Outcome::Draw));

    board_test_main(&board);
}

#[test]
#[ignore]
fn go_perft_19() {
    let rules = Rules::tromp_taylor();
    let board = GoBoard::new(19, rules);

    println!("{}", board);

    // TODO some of these might not consider a double pass to be game over
    // TODO these are probably still allowing repetition
    let all_expected = [1, 362, 130_683, 47_046_604, 16_890_120_013];
    let mut all_correct = true;

    for (depth, &expected) in all_expected.iter().enumerate() {
        let value = perf_naive(&board, depth as u32);
        println!("Perft {}: {} ?= {}", depth, value, expected);

        all_correct &= value == expected;
    }

    assert!(all_correct);
}

#[test]
#[ignore]
fn go_perft_5() {
    let rules = Rules::tromp_taylor();
    let board = GoBoard::new(5, rules);

    println!("{}", board);

    // TODO some of these might not consider a double pass to be game over
    // TODO these are probably still allowing repetition
    let all_expected = [1, 26, 651, 15650, 361233, 7992928, 169263152, 3424697296];
    let mut all_correct = true;

    for (depth, &expected) in all_expected.iter().enumerate() {
        let value = perft(&board, depth as u32);
        println!("Perft {}: {} ?= {}", depth, value, expected);

        all_correct &= value == expected;
    }

    assert!(all_correct);
}
