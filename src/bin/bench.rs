#![allow(clippy::assertions_on_constants)]

use std::time::Instant;

use board_game::ai::mcts::mcts_build_tree;
use board_game::games::ataxx::AtaxxBoard;
use board_game::games::chess::ChessBoard;
use board_game::games::sttt::STTTBoard;
use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn main() {
    bench("mcts_sttt", || {
        mcts_build_tree(&STTTBoard::default(), 100_000, 2.0, &mut SmallRng::from_entropy());
    });

    bench("mcts_ataxx", || {
        mcts_build_tree(&AtaxxBoard::default(), 10_000, 2.0, &mut SmallRng::from_entropy());
    });

    bench("mcts_chess", || {
        mcts_build_tree(&ChessBoard::default(), 1_000, 2.0, &mut SmallRng::from_entropy());
    });
}

const ITERATION_COUNT: usize = 10;
const REMOVED_OUTLIERS_PER_SIDE: usize = 1;

fn bench(name: &str, mut f: impl FnMut()) {
    assert!(ITERATION_COUNT > REMOVED_OUTLIERS_PER_SIDE * 2);
    println!("Running benchmark {}", name);

    // benchmark function
    let mut timings = vec![];

    for _ in 0..ITERATION_COUNT {
        let start = Instant::now();
        f();

        let end = Instant::now();
        timings.push(end - start);
    }

    // remove outliers
    for _ in 0..REMOVED_OUTLIERS_PER_SIDE {
        timings.remove(timings.iter().position_min().unwrap());
        timings.remove(timings.iter().position_max().unwrap());
    }

    // print results
    let timings = timings.iter().map(|d| d.as_secs_f32()).collect_vec();
    let mean = timings.iter().sum::<f32>() / timings.len() as f32;
    let stddev = (timings.iter().map(|&f| (f - mean).powi(2)).sum::<f32>() / timings.len() as f32).sqrt();

    println!("  {:.2}ms\t +- {:.2}ms", mean, stddev);
}
