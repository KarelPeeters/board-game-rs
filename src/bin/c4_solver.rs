use std::cmp::{max, min};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

use internal_iterator::InternalIterator;
use itertools::Itertools;

use board_game::board::{Board, BoardMoves, Outcome};
use board_game::games::connect4::Connect4;
use board_game::util::board_gen::board_with_moves;

fn main() {
    let test_sets = vec![
        ("Test_custom", "Custom"),
        ("Test_L3_R1", "End-Easy"),
        ("Test_L2_R1", "Middle-Easy"),
        ("Test_L2_R2", "Middle-Medium"),
        ("Test_L1_R1", "Begin-Easy"),
        ("Test_L1_R2", "Begin-Medium"),
        ("Test_L1_R3", "Begin-Hard"),
    ];

    let mut cache = Cache::new(8 * 1024 * 1024);

    for &(path, name) in &test_sets {
        for strong in [false, true] {
            let strong_str = if strong { "strong" } else { "weak" };

            let mut total = 0;
            let mut total_correct = 0;
            let mut total_nodes = 0;
            let mut total_hits = 0;
            let mut total_time = Duration::default();

            let file = File::open(Path::new("ignored/connect4_tests").join(path)).unwrap();
            for line in BufReader::new(file).lines() {
                let line = line.unwrap();
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let (moves_str, expected_eval): (&str, &str) = line.split(' ').collect_tuple().unwrap();

                let moves = moves_str
                    .chars()
                    .map(|c| (c.to_digit(10).unwrap() - 1) as u8)
                    .collect_vec();
                let board = board_with_moves(Connect4::default(), &moves);
                let expected_strong_eval = expected_eval.parse::<i32>().unwrap();

                let depth = moves.len() as i32;
                let mut nodes = 0;
                let mut hits = 0;

                cache.clear();
                let start = Instant::now();

                let (expected_eval, eval) = if strong {
                    (
                        expected_strong_eval,
                        solve(&board, -i32::MAX, i32::MAX, &mut cache, depth, &mut nodes, &mut hits),
                    )
                } else {
                    (
                        expected_strong_eval.signum(),
                        solve(&board, -1, 1, &mut cache, depth, &mut nodes, &mut hits),
                    )
                };

                total += 1;
                total_nodes += nodes;
                total_hits += hits;
                total_time += start.elapsed();

                if eval == expected_eval {
                    total_correct += 1;
                } else {
                    eprintln!(
                        "Wrong {} eval on {}, got {} expected {} on board\n{}",
                        strong_str, moves_str, eval, expected_eval, board
                    );
                    debug_board(&board, depth);
                    return;
                }
            }

            let mean_time = total_time / total;
            let mean_nodes = total_nodes as f32 / total as f32;
            let mean_hits = total_hits as f32 / total as f32;
            let correct = total_correct as f32 / total as f32;

            println!(
                "{:<17} {:<8} time: {:>16.4?}   nodes: {:>14.2}   hits: {:>10.2}   correct: {:<4}",
                name, strong_str, mean_time, mean_nodes, mean_hits, correct
            );
        }
    }
}

const SIZE: i32 = (Connect4::WIDTH * Connect4::HEIGHT) as i32;

struct Cache(Vec<u64>);

impl Cache {
    fn new(size: usize) -> Cache {
        let mut result = Cache(vec![0; size]);
        result.clear();
        result
    }

    fn clear(&mut self) {
        self.0.fill(0)
    }

    fn insert(&mut self, board: &Connect4, eval: i32) {
        let hash = board.perfect_hash();
        let index = (hash % self.0.len() as u64) as usize;

        self.0[index] = hash | (eval as i8 as u64) << (64 - 8);
    }

    fn get(&mut self, board: &Connect4) -> Option<i32> {
        let hash = board.perfect_hash();
        let index = (hash % self.0.len() as u64) as usize;

        let value = self.0[index];
        let actual_hash = (value << 8) >> 8;
        let eval = (value >> (64 - 8)) as i8 as i32;

        if hash == actual_hash {
            Some(eval)
        } else {
            None
        }
    }
}

pub const MIN_POSSIBLE_SCORE: i32 = -(Connect4::WIDTH as i32 * Connect4::HEIGHT as i32) / 2 + 3;
pub const MAX_POSSIBLE_SCORE: i32 = (Connect4::WIDTH as i32 * Connect4::HEIGHT as i32 + 1) / 2 - 3;

fn solve(
    board: &Connect4,
    mut alpha: i32,
    mut beta: i32,
    cache: &mut Cache,
    depth: i32,
    nodes: &mut u64,
    hits: &mut u64,
) -> i32 {
    *nodes += 1;

    if let Some(outcome) = board.outcome() {
        return match outcome {
            Outcome::Draw => 0,
            Outcome::WonBy(_) => -(SIZE + 2 - depth) / 2,
        };
    }

    if board
        .available_moves()
        .any(|mv| board.clone_and_play(mv).outcome() == Some(Outcome::WonBy(board.next_player())))
    {
        return (SIZE + 1 - depth) / 2;
    }

    let mut best_possible = (SIZE + 1 - depth) / 2;

    if let Some(eval) = cache.get(board) {
        best_possible = eval + MIN_POSSIBLE_SCORE - 1;
        *hits += 1;
    }

    beta = min(beta, best_possible);
    if alpha >= beta {
        return beta;
    }

    for mv in [3, 2, 4, 1, 5, 0, 6] {
        if !board.is_available_move(mv) {
            continue;
        }

        let score = -solve(&board.clone_and_play(mv), -beta, -alpha, cache, depth + 1, nodes, hits);
        if score >= beta {
            return score;
        }

        alpha = max(alpha, score);
    }

    cache.insert(board, alpha - MIN_POSSIBLE_SCORE + 1);
    return alpha;
}

fn debug_board(board: &Connect4, depth: i32) {
    let mut cache = Cache::new(1);

    if !board.is_done() {
        board.available_moves().for_each(|mv| {
            let child_value = -solve(
                &board.clone_and_play(mv),
                -i32::MAX,
                i32::MAX,
                &mut cache,
                depth,
                &mut 0,
                &mut 0,
            );
            println!("{} => {}", mv, child_value);
        })
    }
}
