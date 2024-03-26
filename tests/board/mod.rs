use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::FromIterator;
use std::time::Instant;

use internal_iterator::InternalIterator;

use board_game::board::{Board, BoardDone, PlayError};
use board_game::symmetry::Symmetry;
use board_game::util::game_stats;
use board_game::util::tiny::consistent_rng;

use crate::util::test_sampler_uniform;

#[cfg(feature = "game_arimaa")]
mod arimaa;
#[cfg(feature = "game_ataxx")]
mod ataxx;
#[cfg(feature = "game_chess")]
mod chess;
#[cfg(feature = "game_connect4")]
mod connect4;
#[cfg(feature = "game_go")]
mod go;
#[cfg(feature = "game_go")]
mod go_chains;

mod max_moves;

#[cfg(feature = "game_oware")]
mod oware;
#[cfg(feature = "game_sttt")]
mod sttt;
#[cfg(feature = "game_ttt")]
mod ttt;

// TODO add test for symmetry after playing moves
pub fn board_test_main<B: Board>(board: &B)
where
    B::Move: Hash,
{
    board_test_main_impl(board, true)
}

pub fn board_test_main_without_uniform<B: Board>(board: &B)
where
    B::Move: Hash,
{
    board_test_main_impl(board, false)
}

fn board_test_main_impl<B: Board>(board: &B, random_uniform: bool)
where
    B::Move: Hash,
{
    println!("Currently testing board\n{:?}\n{}", board, board);

    if board.is_done() {
        test_done_board_errors(board);
    } else {
        test_available_match(board);

        if random_uniform {
            test_random_available_uniform(board);
        }
    }

    test_symmetry(board);
}

pub fn board_perft_main<S: Debug + ?Sized, T: Debug, B: Board + Hash>(
    f: impl Fn(&S) -> B,
    r: Option<impl Fn(&B) -> T>,
    cases: Vec<(&S, Vec<u64>)>,
) where
    for<'a> &'a S: PartialEq<T>,
{
    let total_start = Instant::now();

    for (desc, expected_perfts) in cases {
        let board = f(desc);
        println!("Parsed {:?} as", desc);
        println!("{}", board);

        if let Some(r) = &r {
            assert_eq!(desc, r(&board), "Description mismatch");
        }

        for (depth, &expected_perft) in expected_perfts.iter().enumerate() {
            let curr_start = Instant::now();
            let perft = game_stats::perft_naive(&board, depth as u32);
            let elapsed = curr_start.elapsed();
            println!(
                "   depth {} -> {} =? {}, took {:?}, {} nps",
                depth,
                expected_perft,
                perft,
                elapsed,
                perft as f32 / elapsed.as_secs_f32(),
            );
            assert_eq!(expected_perft, perft)
        }
    }

    println!("Total: took {:?}", total_start.elapsed());
}

fn test_done_board_errors<B: Board>(board: &B) {
    assert!(board.is_done(), "bug in test implementation, expected done board");

    assert!(matches!(board.available_moves(), Err(BoardDone)));
    assert!(matches!(
        board.random_available_move(&mut consistent_rng()),
        Err(BoardDone)
    ));

    B::all_possible_moves().for_each(|mv: B::Move| {
        assert!(matches!(board.clone().play(mv), Err(PlayError::BoardDone)));
        assert!(matches!(board.clone().play(mv), Err(PlayError::BoardDone)));
        assert!(matches!(board.is_available_move(mv), Err(BoardDone)));
    });
}

fn test_available_match<B: Board>(board: &B)
where
    B::Move: Hash,
{
    println!("available_moves and is_available match:");

    let all: Vec<B::Move> = B::all_possible_moves().collect();
    let available: Vec<B::Move> = board.available_moves().unwrap().collect();

    let all_count = B::all_possible_moves().count();
    let available_count = board.available_moves().unwrap().count();

    assert_eq!(all.len(), all_count, "all_possible_moves count mismatch");
    assert_eq!(available.len(), available_count, "available_moves count mismatch");

    assert!(
        !available.is_empty(),
        "must have at least one available move for non-done board"
    );

    // check that every generated move is indeed available, and that it is contained within all possible moves
    for &mv in &available {
        assert!(
            board.is_available_move(mv).unwrap(),
            "generated move {:?} is not available",
            mv
        );
        assert!(
            all.contains(&mv),
            "generated move {:?} is not in all_possible_moves",
            mv
        );
    }

    // check that every available move is generated
    for &mv in &all {
        if board.is_available_move(mv).unwrap() {
            assert!(available.contains(&mv), "available move {:?} was not generated", mv);
        } else {
            assert!(!available.contains(&mv), "non-available move {:?} was generated", mv)
        }
    }

    // check that there are no duplicates anywhere
    assert_eq!(
        all.len(),
        HashSet::<_, RandomState>::from_iter(&all).len(),
        "Found duplicate move"
    );
    assert_eq!(
        available.len(),
        HashSet::<_, RandomState>::from_iter(&available).len(),
        "Found duplicate move"
    );

    // try playing each available move
    for &mv in &available {
        println!("Playing {}", mv);
        println!("{}", board.clone_and_play(mv).unwrap());
    }
}

/// Test whether the random move distribution is uniform using
/// [Pearson's chi-squared test](https://en.wikipedia.org/wiki/Pearson%27s_chi-squared_test).
fn test_random_available_uniform<B: Board>(board: &B)
where
    B::Move: Hash,
{
    assert!(!board.is_done(), "invalid board to test");

    println!("random_available uniform:");
    println!("{}", board);

    let expected: Vec<_> = board.available_moves().unwrap().collect();
    let mut rng = consistent_rng();
    test_sampler_uniform(&expected, true, || Some(board.random_available_move(&mut rng).unwrap()));
}

fn test_symmetry<B: Board>(board: &B)
where
    B::Move: Hash,
{
    // TODO test that maps, plays move, then maps back
    //  this checks that moves and board state are really mapped correctly
    println!("symmetries:");

    let all = B::Symmetry::all();
    assert!(all.contains(&B::Symmetry::default()));

    for &sym in B::Symmetry::all() {
        let sym_inv = sym.inverse();

        println!("{:?}", sym);
        println!("inverse: {:?}", sym_inv);

        assert!(all.contains(&sym_inv));

        let mapped = board.map(sym);
        let back = mapped.map(sym_inv);

        // these prints test that the board is consistent enough to print it
        println!("Mapped:\n{}", mapped);
        println!("Back:\n{}", back);

        if sym == B::Symmetry::default() {
            assert_eq!(board, &mapped);
        }
        assert_eq!(board, &back);

        assert_eq!(board.outcome(), mapped.outcome());
        assert_eq!(board.next_player(), mapped.next_player());

        if !board.is_done() {
            let expected_moves_shuffled: Vec<B::Move> = board
                .available_moves()
                .unwrap()
                .map(|c| board.map_move(sym, c))
                .collect();
            let actual_moves_shuffled: Vec<B::Move> = mapped.available_moves().unwrap().collect();

            let expected_moves = sort_moves::<B>(&expected_moves_shuffled);
            let actual_moves = sort_moves::<B>(&actual_moves_shuffled);

            assert_eq!(expected_moves, actual_moves);

            for mv in actual_moves {
                assert!(mapped.is_available_move(mv).unwrap());
            }
        }
    }

    // run in separate loop so we already know symmetries work
    let expected_canonical = board.canonicalize();
    for &sym in B::Symmetry::all() {
        assert_eq!(expected_canonical, board.map(sym).canonicalize());
    }
}

fn sort_moves<B: Board>(moves: &[B::Move]) -> Vec<B::Move> {
    B::all_possible_moves().filter(|&mv| moves.contains(&mv)).collect()
}

pub fn print_board_with_moves<B: Board>(start: B, moves: &[B::Move]) -> B {
    let mut board = start;
    println!("{}", board);

    for &mv in moves {
        println!("Playing {}", mv);
        board.play(mv).unwrap();
        println!("{}", board);
    }

    board
}
