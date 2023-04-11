use board_game::board::{Board, Outcome};
use board_game::games::go::{GoBoard, Move, Rules, Tile};
use board_game::util::board_gen::board_with_moves;
use board_game::util::game_stats::{perf_naive, perft};

use crate::board::go_chains::chains_test_main;
use crate::board::{board_perft_main, print_board_with_moves};

#[test]
fn tile() {
    let cases = [
        // basic
        ((0, 0), "A1"),
        ((1, 0), "B1"),
        ((0, 1), "A2"),
        // i skipped
        ((7, 0), "H1"),
        ((8, 0), "J1"),
        ((9, 0), "K1"),
        // largest 19x19 tile
        ((0, 18), "A19"),
        ((18, 0), "T1"),
        ((18, 18), "T19"),
        // largest tile
        ((24, 24), "Z25"),
    ];

    for ((x, y), s) in cases {
        let tile = Tile::new(x, y);
        assert_eq!(tile.to_string(), s);
        assert_eq!(tile, s.parse().unwrap());
    }
}

#[test]
#[ignore]
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

        go_board_test_main(&board);
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
    assert_eq!(board.stone_at(Tile::new(0, 0)), None);

    go_board_test_main(&board);
}

#[test]
fn double_pass() {
    let rules = Rules::tromp_taylor();
    let start = GoBoard::new(5, rules);
    let moves = [Move::Pass, Move::Pass];

    let board = print_board_with_moves(start, &moves);
    assert_eq!(board.outcome(), Some(Outcome::Draw));

    go_board_test_main(&board);
}

fn simulate_moves(start: &str, moves: &[Move], result: &str) {
    let rules = Rules::tromp_taylor();
    let board = print_board_with_moves(GoBoard::from_fen(start, rules).unwrap(), moves);

    let result = GoBoard::from_fen(result, rules).unwrap();
    println!("Expected:\n{}", result);

    // TODO find a better way to compare without considering history
    assert_eq!(board.to_fen(), result.to_fen());

    go_board_test_main(&board);
}

#[test]
fn capture_large() {
    simulate_moves(
        ".w.../wbw../bbbw./wbb../.ww.. w 0",
        &[Move::Place(Tile::new(3, 1))],
        ".w.../w.w../...w./w..w./.ww.. b 0",
    );
}

#[test]
fn capture_inner() {
    simulate_moves(
        "...../.w.../wbw../b.bw./bbw.. w 0",
        &[Move::Place(Tile::new(1, 1))],
        "...../.w.../w.w../.w.w./..w.. b 0",
    );
}

#[test]
fn self_capture() {
    simulate_moves(
        "...../.w.../wbw../b.bw./bbw.. b 0",
        &[Move::Place(Tile::new(1, 1))],
        "...../.w.../w.w../...w./..w.. w 0",
    );
}

#[test]
fn double_eye() {
    let start = "...../...../wwwww/bbbbb/.b.bb w 0";
    let end = "...../...../wwwww/bbbbb/.b.bb b 0";

    simulate_moves(start, &[Move::Place(Tile::new(0, 0))], end);
    simulate_moves(start, &[Move::Place(Tile::new(2, 0))], end);
}

#[test]
fn suicide_1() {
    let start = "...../...../...../b..../.b... w 0";
    let mv = Move::Place(Tile::new(0, 0));

    let board = GoBoard::from_fen(start, Rules::tromp_taylor()).unwrap();
    println!("{}", board);

    // not allowed, would immediately repeat
    assert_eq!(Ok(false), board.is_available_move(mv));

    go_board_test_main(&board);
}

#[test]
fn suicide_2() {
    let start = "...../...../b..../wb.../.b... w 0";
    let mv = Move::Place(Tile::new(0, 0));

    let board_tt = GoBoard::from_fen(start, Rules::tromp_taylor()).unwrap();
    let board_cgos = GoBoard::from_fen(start, Rules::cgos()).unwrap();
    println!("{}", board_tt);

    // allowed in TT, does not repeat (yet)
    assert_eq!(Ok(true), board_tt.is_available_move(mv));
    // not allowed in CGOS, suicide is banned
    assert_eq!(Ok(false), board_cgos.is_available_move(mv));

    // TODO set up repeating situation that is disallowed by TT

    go_board_test_main(&board_tt);
    go_board_test_main(&board_cgos);
}

#[test]
fn super_ko() {
    // TODO write superko test
}

#[test]
#[ignore]
fn go_perft_19() {
    let rules = Rules::tromp_taylor();
    let board = GoBoard::new(19, rules);

    println!("{}", board);

    // TODO some of these might not consider a double pass to be game over
    // TODO these are probably still allowing repetition
    let all_expected = [1, 362, 130683, 47046604, 16890120013];
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

#[test]
fn go_perft_fast() {
    let rules = Rules::tromp_taylor();

    board_perft_main(
        |s| GoBoard::from_fen(s, rules).unwrap(),
        Some(GoBoard::to_fen),
        vec![
            ("...../...../...../...../..... b 0", vec![1, 26, 651, 15650, 361233]),
            ("...../...../...../...b./..b.b w 0", vec![1, 23, 508, 10715, 216332]),
        ],
    );
}

fn go_board_test_main(board: &GoBoard) {
    chains_test_main(board.chains(), &board.rules());
    crate::board::board_test_main(board);
}
