use internal_iterator::InternalIterator;
use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use board_game::board::{Board, BoardMoves, BoardSymmetry, Outcome, Player};
use board_game::games::ataxx::{ataxx_back_perft, AtaxxBoard, BackMove, Move, PrevTerminal};
use board_game::symmetry::D4Symmetry;
use board_game::util::bitboard::BitBoard8;
use board_game::util::board_gen::random_board_with_moves;
use board_game::util::game_stats::perft;

use crate::board::{board_perft_main, board_test_main};

#[test]
fn ataxx_empty() {
    for size in 0..AtaxxBoard::MAX_SIZE {
        println!("Size: {}", size);
        board_test_main(&AtaxxBoard::empty(size))
    }
}

#[test]
fn ataxx_sizes() {
    let pairs = [
        (2, "xo/ox x 0 1"),
        (3, "x1o/3/o1x x 0 1"),
        (4, "x2o/4/4/o2x x 0 1"),
        (5, "x3o/5/5/5/o3x x 0 1"),
        (6, "x4o/6/6/6/6/o4x x 0 1"),
        (7, "x5o/7/7/7/7/7/o5x x 0 1"),
        (8, "x6o/8/8/8/8/8/8/o6x x 0 1"),
    ];

    for (size, fen) in pairs {
        let actual = AtaxxBoard::diagonal(size);
        let expected = AtaxxBoard::from_fen(fen).unwrap();
        println!("{}", actual);
        println!("{}", expected);
        assert_eq!(actual, expected);

        board_test_main(&actual);
    }
}

#[test]
fn ataxx_check_tile_count() {
    let mut rng = SmallRng::seed_from_u64(0);
    for size in 5..AtaxxBoard::MAX_SIZE {
        for _ in 0..100 {
            let board = random_board_with_moves(&AtaxxBoard::diagonal(size), rng.gen_range(0..20), &mut rng);

            let board_str = board.to_string();
            println!("{}", board_str);

            let x_bonus = (board.next_player() == Player::A) as usize;
            let o_bonus = (board.next_player() == Player::B) as usize;

            assert_eq!(
                (board.tiles_a().count() as usize + x_bonus) * 2,
                board_str.matches('x').count()
            );
            assert_eq!(
                (board.tiles_b().count() as usize + o_bonus) * 2,
                board_str.matches('o').count()
            );
        }
    }
}

#[test]
fn ataxx_diag() {
    for size in 2..AtaxxBoard::MAX_SIZE {
        println!("Size: {}", size);
        board_test_main(&AtaxxBoard::diagonal(size))
    }
}

#[test]
fn ataxx_few() {
    board_test_main(&AtaxxBoard::from_fen("2x3o/1x3oo/7/7/7/7/o3x2 o 0 1").unwrap());
}

#[test]
fn ataxx_close() {
    let board = AtaxxBoard::from_fen("ooooooo/xxxxooo/oxxxoo1/oxxxooo/ooxoooo/xxxxxoo/xxxxxxx x 0 1").unwrap();
    assert!(!board.is_done());
    board_test_main(&board)
}

#[test]
fn ataxx_done_clear() {
    let board = AtaxxBoard::from_fen("4x2/4xx1/xxx4/1x5/4x2/7/7 o 2 1").unwrap();
    assert_eq!(Some(Outcome::WonBy(Player::A)), board.outcome());
    board_test_main(&board)
}

#[test]
fn ataxx_done_full() {
    let board = AtaxxBoard::from_fen("xxxoxxx/ooooxxx/ooooxxx/xxxooox/xxxooox/xxxxxxx/ooooxxx o 0 1").unwrap();
    assert_eq!(Some(Outcome::WonBy(Player::A)), board.outcome());
    assert!(!board.must_pass());
    board_test_main(&board)
}

#[test]
fn ataxx_forced_pass() {
    let board = AtaxxBoard::from_fen("xxxxxxx/-------/-------/o6/7/7/7 x 0 0").unwrap();
    assert!(!board.is_done(), "Board is not done, player B can still play");
    assert!(board.must_pass());
    assert_eq!(board.available_moves().unwrap().collect::<Vec<_>>(), vec![Move::Pass]);
    board_test_main(&board)
}

#[test]
fn ataxx_2() {
    board_test_main(&AtaxxBoard::diagonal(2));
    board_test_main(&AtaxxBoard::from_fen("x1/1o x 0 0").unwrap());
}

#[test]
fn ataxx_2_sym() {
    let board = AtaxxBoard::from_fen("x1/1o x 0 0").unwrap();
    let expected = AtaxxBoard::from_fen("1x/o1 x 0 0").unwrap();

    println!("{}", board);
    println!("{}", expected);

    let sym = D4Symmetry::new(false, true, false);
    let actual = board.map(sym);
    println!();
    println!("{}", actual);

    println!("board a:    \n{}", board.tiles_a());
    println!("board b:    \n{}", board.tiles_b());
    println!("expected a: \n{}", expected.tiles_a());
    println!("expected b: \n{}", expected.tiles_b());
    println!("actual b:   \n{}", actual.tiles_b());
    println!("actual a:   \n{}", actual.tiles_a());

    print!("board a tiles: {:?}", board.tiles_a().into_iter().collect_vec());

    for c in board.tiles_a() {
        println!("a coord {}, mapped {}", c, sym.map_coord(c, board.size()));
    }

    println!("mapped a tiles: \n{}", board.map_tiles(board.tiles_a(), sym));
    assert_eq!(actual, expected);
}

#[test]
fn ataxx_5_pass() {
    let board = AtaxxBoard::from_fen("xoooo/xoo2/ooooo/ooooo/xxooo x 0 0").unwrap();

    assert_eq!(board.available_moves().unwrap().count(), 1);
    assert!(board.is_available_move(Move::Pass).unwrap());

    board_test_main(&board);
}

#[test]
fn ataxx_8() {
    board_test_main(&AtaxxBoard::diagonal(8));
    board_test_main(&AtaxxBoard::from_fen("7x/2x4x/8/8/8/o4o2/o7/o6o o 0 0").unwrap());
}

#[test]
fn ataxx_pass_move_counter() {
    let mut board = AtaxxBoard::from_fen("xxxxxxx/-------/-------/o6/7/7/7 x 0 0").unwrap();
    board_test_main(&board);

    let prev_count = board.moves_since_last_copy();
    board.play(Move::Pass).unwrap();
    assert_eq!(board.moves_since_last_copy(), prev_count + 1);
}

#[test]
fn ataxx_copy_move_counter() {
    let mut board = AtaxxBoard::from_fen("x2xxxx/-------/-------/o6/7/7/7 x 0 0").unwrap();

    board.play(Move::from_uai("b7").unwrap()).unwrap();
    assert_eq!(board.moves_since_last_copy(), 0);

    board_test_main(&board);
}

#[test]
fn ataxx_jump_move_counter() {
    let mut board = AtaxxBoard::from_fen("x2xxxx/-------/-------/o6/7/7/7 x 0 0").unwrap();

    let prev_count = board.moves_since_last_copy();
    board.play(Move::from_uai("a7c7").unwrap()).unwrap();
    assert_eq!(board.moves_since_last_copy(), prev_count + 1);

    board_test_main(&board);
}

///Test cases from <https://github.com/kz04px/libataxx/blob/master/tests/perft.cpp>, edited to remove move counters.
#[test]
fn ataxx_perft() {
    #[rustfmt::skip]
    board_perft_main(
        |s| AtaxxBoard::from_fen(s).unwrap(),
        Some(AtaxxBoard::to_fen),
        perft,
        vec![
            ("7/7/7/7/7/7/7 x 0 1", vec![1, 0, 0, 0, 0]),
            ("7/7/7/7/7/7/7 o 0 1", vec![1, 0, 0, 0, 0]),
            ("x5o/7/7/7/7/7/o5x x 0 1", vec![1, 16, 256, 6460, 155888, 4752668]),
            ("x5o/7/7/7/7/7/o5x o 0 1", vec![1, 16, 256, 6460, 155888, 4752668]),
            ("x5o/7/2-1-2/7/2-1-2/7/o5x x 0 1", vec![1, 14, 196, 4184, 86528, 2266352]),
            ("x5o/7/2-1-2/7/2-1-2/7/o5x o 0 1", vec![1, 14, 196, 4184, 86528, 2266352]),
            ("x5o/7/2-1-2/3-3/2-1-2/7/o5x x 0 1", vec![1, 14, 196, 4100, 83104, 2114588]),
            ("x5o/7/2-1-2/3-3/2-1-2/7/o5x o 0 1", vec![1, 14, 196, 4100, 83104, 2114588]),
            ("x5o/7/3-3/2-1-2/3-3/7/o5x x 0 1", vec![1, 16, 256, 5948, 133264, 3639856]),
            ("x5o/7/3-3/2-1-2/3-3/7/o5x o 0 1", vec![1, 16, 256, 5948, 133264, 3639856]),
            ("7/7/7/7/ooooooo/ooooooo/xxxxxxx x 0 1", vec![1, 1, 75, 249, 14270, 452980]),
            ("7/7/7/7/ooooooo/ooooooo/xxxxxxx o 0 1", vec![1, 75, 249, 14270, 452980]),
            ("7/7/7/7/xxxxxxx/xxxxxxx/ooooooo x 0 1", vec![1, 75, 249, 14270, 452980]),
            ("7/7/7/7/xxxxxxx/xxxxxxx/ooooooo o 0 1", vec![1, 1, 75, 249, 14270, 452980]),
            ("7/7/7/2x1o2/7/7/7 x 0 1", vec![1, 23, 419, 7887, 168317, 4266992]),
            ("7/7/7/2x1o2/7/7/7 o 0 1", vec![1, 23, 419, 7887, 168317, 4266992]),
            ("x5o/7/7/7/7/7/o5x x 100 1", vec![1, 0, 0, 0, 0]),
            ("x5o/7/7/7/7/7/o5x o 100 1", vec![1, 0, 0, 0, 0]),
            ("7/7/7/7/-------/-------/x5o x 0 1", vec![1, 2, 4, 13, 30, 73, 174]),
            ("7/7/7/7/-------/-------/x5o o 0 1", vec![1, 2, 4, 13, 30, 73, 174]),
        ],
    );
}

#[test]
fn ataxx_back_pass() {
    let board = AtaxxBoard::from_fen("oxxxooo/oxxxooo/ooxxxoo/ooxxxoo/xxxxxoo/xxxxxxx/xx1xxoo x 0 0").unwrap();
    println!("{}", board);

    let back_moves = board.back_moves().collect::<Vec<_>>();
    for back in &back_moves {
        println!("  {:?}", back);
    }

    assert!(back_moves.contains(&BackMove::PASS));
}

#[test]
fn ataxx_back_terminal() {
    let mut board = AtaxxBoard::from_fen("x1x2/1x3/1x3/5/4x o 0 1").unwrap();
    println!("{}", board);

    // try a move that still yields a terminal move
    let back = BackMove {
        mv: Move::from_uai("b4").unwrap(),
        converted: BitBoard8::EMPTY,
    };
    let result = board.play_back(back);
    assert_eq!(result, Err(PrevTerminal));
}

#[test]
fn ataxx_perft_back() {
    #[rustfmt::skip]
    board_perft_main(
        |s| AtaxxBoard::from_fen(s).unwrap(),
        Some(AtaxxBoard::to_fen),
        ataxx_back_perft,
        vec![
            ("x5o/7/7/7/7/7/o5x x 0 1", vec![1, 10, 100, 1584, 23132, 350092, 4978660, 77305740]),
            ("6o/7/7/7/7/xx5/xx5 o 0 1", vec![1, 220, 2375, 37766, 609479, 11097618, 190111539]),
            ("6o/7/7/7/7/7/xx5 o 0 1", vec![1, 24, 162, 1868, 22874, 308829, 3675210, 44830928]),
            ("oxxxooo/oxxxooo/ooxxxoo/ooxxxoo/xxxxxoo/xxxxxxx/xx1xxoo x 0 1", vec![1, 132, 188826, 11199385]),
            ("-------/-------/x1x2--/1x3--/1x3--/5--/4x-- o 0 1", vec![1, 109, 405, 6127, 56898, 910186, 8979185]),
        ],
    );
}
