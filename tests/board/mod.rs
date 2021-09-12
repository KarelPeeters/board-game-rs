use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;
use std::panic::catch_unwind;

use internal_iterator::InternalIterator;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoroshiro64StarStar;

use board_game::board::Board;
use board_game::symmetry::Symmetry;

mod ataxx;
mod chess;
mod sttt;

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

    for &sym in B::Symmetry::all() {
        let sym_inv = sym.inverse();

        println!("{:?}", sym);
        println!("inverse: {:?}", sym_inv);

        let mapped = board.map(sym);
        let back = mapped.map(sym_inv);

        // these prints test that the board is consistent enough to print it
        println!("Mapped:\n{}", mapped);
        println!("Back:\n{}", back);

        if sym == B::Symmetry::identity() {
            assert_eq!(board, &mapped);
        }
        assert_eq!(board, &back);

        assert_eq!(board.outcome(), mapped.outcome());
        assert_eq!(board.next_player(), mapped.next_player());

        if !board.is_done() {
            let mut expected_moves: Vec<B::Move> = board.available_moves().map(|c| B::map_move(sym, c)).collect();
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
