use board_game::board::{Board, Outcome, PlayError};
use board_game::games::go::{GoBoard, Move, Rules, Tile};
use board_game::util::board_gen::board_with_moves;
use board_game::util::game_stats::perft_naive;
use std::str::FromStr;

use crate::board::go_chains::chains_test_main;
use crate::board::print_board_with_moves;

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
        let board = board.without_history();

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

fn simulate_moves(start: &str, moves: &[Move], result: &str, rules: Rules) {
    let start_board = GoBoard::from_fen(start, rules).unwrap();
    let result_board = print_board_with_moves(start_board, moves);

    let result_board_expected = GoBoard::from_fen(result, rules).unwrap();
    println!("Expected:\n{}", result_board_expected);
    assert_eq!(result_board.without_history(), result_board_expected);

    go_board_test_main(&result_board);
}

#[test]
fn capture_large() {
    simulate_moves(
        ".w.../wbw../bbbw./wbb../.ww.. w 0",
        &[Move::Place(Tile::new(3, 1))],
        ".w.../w.w../...w./w..w./.ww.. b 0",
        Rules::tromp_taylor(),
    );
}

#[test]
fn capture_inner() {
    simulate_moves(
        "...../.w.../wbw../b.bw./bbw.. w 0",
        &[Move::Place(Tile::new(1, 1))],
        "...../.w.../w.w../.w.w./..w.. b 0",
        Rules::tromp_taylor(),
    );
}

#[test]
fn self_capture() {
    simulate_moves(
        "...../.w.../wbw../b.bw./bbw.. b 0",
        &[Move::Place(Tile::new(1, 1))],
        "...../.w.../w.w../...w./..w.. w 0",
        Rules::tromp_taylor(),
    );
}

#[test]
fn double_eye() {
    let fen = "...../...../wwwww/bbbbb/.b.bb w 0";
    let board = GoBoard::from_fen(fen, Rules::tromp_taylor()).unwrap();

    let mv_left = Move::Place(Tile::new(0, 0));
    let mv_right = Move::Place(Tile::new(2, 0));

    // single-stone suicide is not allowed
    assert_eq!(Ok(false), board.is_available_move(mv_left));
    assert_eq!(Ok(false), board.is_available_move(mv_right));

    go_board_test_main(&board);
}

#[test]
fn suicide_1() {
    let start = "...../...../...../b..../.b... w 0";
    let mv = Move::Place(Tile::new(0, 0));

    let board = GoBoard::from_fen(start, Rules::tromp_taylor()).unwrap();
    println!("{}", board);

    // not allowed, would immediately repeat
    assert_eq!(Ok(false), board.is_available_move(mv));
    assert_eq!(board.clone_and_play(mv), Err(PlayError::UnavailableMove));

    go_board_test_main(&board);
}

#[test]
fn suicide_2() {
    let start = "...../...../b..../wb.../.b... w 0";
    let after = "...../...../b..../.b.../.b... b 0";
    let mv = Move::Place(Tile::new(0, 0));

    // allowed in TT, does not repeat (yet)
    let board_tt = GoBoard::from_fen(start, Rules::tromp_taylor()).unwrap();
    let board_tt_after = GoBoard::from_fen(after, Rules::tromp_taylor()).unwrap();
    println!("{}", board_tt);
    assert_eq!(Ok(true), board_tt.is_available_move(mv));
    assert_eq!(
        Ok(board_tt_after),
        board_tt.clone_and_play(mv).map(|b| b.without_history())
    );

    // not allowed in CGOS, suicide is banned
    let board_cgos = GoBoard::from_fen(start, Rules::cgos()).unwrap();
    println!("{}", board_cgos);
    assert_eq!(Ok(false), board_cgos.is_available_move(mv));
    assert_eq!(Err(PlayError::UnavailableMove), board_cgos.clone_and_play(mv));

    // TODO set up repeating situation that is disallowed by TT

    go_board_test_main(&board_tt);
    go_board_test_main(&board_cgos);
}

#[test]
fn super_ko() {
    // Based on Example from https://senseis.xmp.net/?SuperKo
    let fen = "...bw/wbbbw/w.bww/bbbw./wwww. w 0";
    let mut board = GoBoard::from_fen(fen, Rules::tromp_taylor()).unwrap();
    println!("{}", board);

    let a = Tile::new(2, 4);
    let b = Tile::new(0, 4);
    let mid = Tile::new(1, 4);
    println!("a={:?}, b={:?}, mid={:?}", a, b, mid);

    // everything is available now
    assert_eq!(Ok(true), board.is_available_move(Move::Place(a)));
    assert_eq!(Ok(true), board.is_available_move(Move::Place(b)));
    assert_eq!(Ok(true), board.is_available_move(Move::Place(mid)));

    board.play(Move::Place(mid)).unwrap();
    board.play(Move::Pass).unwrap();
    board.play(Move::Place(a)).unwrap();
    board.play(Move::Place(b)).unwrap();
    println!("{}", board);

    // mid is empty but cannot play, stones would repeat!
    assert_eq!(None, board.stone_at(mid));

    assert_eq!(Ok(true), board.is_available_move(Move::Place(a)));
    assert_eq!(Ok(false), board.is_available_move(Move::Place(b)));
    assert_eq!(Ok(false), board.is_available_move(Move::Place(mid)));
}

#[test]
fn super_ko_repeat() {
    // Example found while debugging 5x5 perft
    let rules = Rules::tromp_taylor();
    let moves = [
        Move::Place(Tile::from_str("A1").unwrap()),
        Move::Pass,
        Move::Place(Tile::from_str("B2").unwrap()),
        Move::Pass,
        Move::Place(Tile::from_str("C2").unwrap()),
        Move::Place(Tile::from_str("B1").unwrap()),
        Move::Pass,
    ];
    let start = GoBoard::new(3, rules);
    let board = print_board_with_moves(start, &moves);

    let fen_before = ".../.bb/bw. w 1";
    assert_eq!(GoBoard::from_fen(fen_before, rules).unwrap(), board.without_history());

    // not available, would repeat earlier pos
    let mv = Move::Place(Tile::from_str("C1").unwrap());
    println!("Checking if {:?} is available", mv);

    assert_eq!(Ok(false), board.is_available_move(mv));
    assert_eq!(Err(PlayError::UnavailableMove), board.clone_and_play(mv));
}

// TODO add profiling
// TODO unify with board_perft_main
fn go_perft_main(board: GoBoard, all_expected: &[u64]) {
    println!("Running perft with {:?} for:", board.rules());
    println!("{}", board);

    let mut all_correct = true;

    for (depth, &expected) in all_expected.iter().enumerate() {
        let value = perft_naive(&board, depth as u32);

        let suffix = if value == expected { "" } else { " -> wrong!" };
        println!("Perft depth {}: expected {} got {}{}", depth, expected, value, suffix);

        all_correct &= value == expected;
    }

    assert!(all_correct);
}

#[test]
#[ignore]
fn go_perft_3() {
    go_perft_main(
        GoBoard::new(3, Rules::tromp_taylor()),
        &[1, 10, 91, 738, 5281, 33384, 180768, 857576, 3474312, 12912040, 44019568],
    );
    go_perft_main(
        GoBoard::new(3, Rules::cgos()),
        &[
            1, 10, 91, 738, 5281, 33384, 179712, 842696, 3271208, 11279096, 33786208, 98049080, 276391080, 783708048,
        ],
    );
}

#[test]
#[ignore]
fn go_perft_5() {
    go_perft_main(
        GoBoard::new(5, Rules::tromp_taylor()),
        &[1, 26, 651, 15650, 361041, 7984104, 168759376, 3407616216],
    );
    go_perft_main(
        GoBoard::new(5, Rules::cgos()),
        &[1, 26, 651, 15650, 361041, 7984104, 168755200, 3407394696],
    );
}

#[test]
#[ignore]
fn go_perft_19() {
    go_perft_main(
        GoBoard::new(19, Rules::tromp_taylor()),
        &[1, 362, 130683, 47046242, 16889859009],
    );
    go_perft_main(
        GoBoard::new(19, Rules::cgos()),
        &[1, 362, 130683, 47046242, 16889859009],
    );
}

#[test]
fn go_perft_fast() {
    // 5x5 empty
    go_perft_main(GoBoard::new(5, Rules::tromp_taylor()), &[1, 26, 651, 15650, 361041]);

    // 5x5 pocket
    go_perft_main(
        GoBoard::from_fen("...../...../...../...b./..b.b w 0", Rules::tromp_taylor()).unwrap(),
        &[1, 22, 485, 9745, 195728],
    );
    go_perft_main(
        GoBoard::from_fen("...../...../...../...b./..b.b w 0", Rules::cgos()).unwrap(),
        &[1, 22, 485, 9745, 195728],
    );

    // 5x5 triple ko
    go_perft_main(
        GoBoard::from_fen(".w.bw/wbbbw/w.bww/bbbw./wwww. b 0", Rules::tromp_taylor()).unwrap(),
        &[1, 5, 26, 121, 925, 8451, 87647],
    );
    go_perft_main(
        GoBoard::from_fen(".w.bw/wbbbw/w.bww/bbbw./wwww. b 0", Rules::cgos()).unwrap(),
        &[1, 5, 26, 109, 739, 6347, 62970],
    );
}

fn go_board_test_main(board: &GoBoard) {
    chains_test_main(board.chains());

    // TODO this is super slow for go
    // crate::board::board_test_main(board);
}
