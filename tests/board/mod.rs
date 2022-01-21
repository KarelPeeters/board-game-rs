use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
use std::iter::FromIterator;
use std::panic::catch_unwind;
use std::time::Instant;

use internal_iterator::InternalIterator;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoroshiro64StarStar;

use board_game::board::Board;
use board_game::symmetry::Symmetry;
use board_game::util::game_stats;

mod ataxx;
mod chess;
mod connect4;
mod max_moves;
mod sttt;
mod ttt;

pub fn board_test_main<B: Board>(board: &B) {
    println!("Currently testing board\n{:?}\n{}", board, board);

    if board.is_done() {
        test_done_board_panics(board);
    } else {
        test_available_match(board);
        test_random_available_uniform(board);
    }

    test_symmetry(board);
}

pub fn board_perft_main<S: Debug + ?Sized, T: Debug, B: Board>(
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
            let perft = game_stats::perft(&board, depth as u32);
            println!(
                "   depth {} -> {} =? {}, took {:?}",
                depth,
                expected_perft,
                perft,
                curr_start.elapsed()
            );
            assert_eq!(expected_perft, perft)
        }
    }

    println!("Total: took {:?}", total_start.elapsed());
}

fn test_done_board_panics<B: Board>(board: &B) {
    assert!(board.is_done(), "bug in test implementation");

    assert!(catch_unwind(|| board.available_moves()).is_err(), "must panic");
    assert!(catch_unwind(|| board.random_available_move(&mut consistent_rng())).is_err());

    B::all_possible_moves().for_each(|mv: B::Move| {
        assert!(catch_unwind(|| board.clone().play(mv)).is_err(), "must panic");
        assert!(catch_unwind(|| board.clone_and_play(mv)).is_err(), "must panic");
        assert!(catch_unwind(|| board.is_available_move(mv)).is_err(), "must panic")
    });
}

fn test_available_match<B: Board>(board: &B) {
    println!("available_moves and is_available match:");

    let all: Vec<B::Move> = B::all_possible_moves().collect();
    let available: Vec<B::Move> = board.available_moves().collect();

    let all_count = B::all_possible_moves().count();
    let available_count = board.available_moves().count();

    assert_eq!(all.len(), all_count, "all_possible_moves count mismatch");
    assert_eq!(available.len(), available_count, "available_moves count mismatch");

    assert!(
        !available.is_empty(),
        "must have at least one available move for non-done board"
    );

    // check that every generated move is indeed available, and that it is contained within all possible moves
    for &mv in &available {
        assert!(board.is_available_move(mv), "generated move {:?} is not available", mv);
        assert!(
            all.contains(&mv),
            "generated move {:?} is not in all_possible_moves",
            mv
        );
    }

    // check that every available move is generated
    for &mv in &all {
        if board.is_available_move(mv) {
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
        println!("{}", board.clone_and_play(mv));
    }
}

/// Test whether the random move distribution is uniform using
/// [Pearson's chi-squared test](https://en.wikipedia.org/wiki/Pearson%27s_chi-squared_test).
fn test_random_available_uniform<B: Board>(board: &B) {
    assert!(!board.is_done(), "invalid board to test");

    println!("random_available uniform:");
    println!("{}", board);

    let mut rng = consistent_rng();

    let available_move_count = board.available_moves().count();
    let total_samples = 1000 * available_move_count;
    let expected_samples = total_samples as f32 / available_move_count as f32;

    println!(
        "Available moves: {}, samples: {}, expected: {}",
        available_move_count, total_samples, expected_samples
    );

    let mut counts: BTreeMap<B::Move, u32> = BTreeMap::new();
    for _ in 0..total_samples {
        let mv = board.random_available_move(&mut rng);
        *counts.entry(mv).or_default() += 1;
    }

    for (&mv, &count) in &counts {
        println!("Move {:?} -> count {} ~ {}", mv, count, count as f32 / expected_samples);
    }

    for (&mv, &count) in &counts {
        assert!(
            (count as f32) > 0.8 * expected_samples,
            "Move {:?} not generated often enough",
            mv
        );
        assert!(
            (count as f32) < 1.2 * expected_samples,
            "Move {:?} generated too often",
            mv
        );
    }
}

fn test_symmetry<B: Board>(board: &B) {
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
            let mut expected_moves: Vec<B::Move> = board.available_moves().map(|c| board.map_move(sym, c)).collect();
            let mut actual_moves: Vec<B::Move> = mapped.available_moves().collect();

            expected_moves.sort();
            actual_moves.sort();

            assert_eq!(expected_moves, actual_moves);

            for mv in actual_moves {
                assert!(mapped.is_available_move(mv));
            }
        }
    }
}

fn consistent_rng() -> impl Rng {
    Xoroshiro64StarStar::seed_from_u64(0)
}
