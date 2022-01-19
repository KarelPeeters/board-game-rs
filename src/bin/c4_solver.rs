use std::cmp::{max, min};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::ControlFlow;
use std::path::Path;
use std::time::Instant;

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

    for &(path, name) in &test_sets {
        for strong in [false, true] {
            let strong_str = if strong { "strong" } else { "weak" };

            let start = Instant::now();
            let mut total = 0;
            let mut total_correct = 0;
            let mut total_nodes = 0;

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

                let (expected_eval, eval) = if strong {
                    (
                        expected_strong_eval,
                        solve(&board, -i32::MAX, i32::MAX, depth, &mut nodes),
                    )
                } else {
                    (expected_strong_eval.signum(), solve(&board, -1, 1, depth, &mut nodes))
                };

                total += 1;
                total_nodes += nodes;

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

            let mean_time = start.elapsed() / total;
            let mean_nodes = total_nodes as f32 / total as f32;
            let correct = total_correct as f32 / total as f32;

            println!(
                "{:<17} {:<8} time: {:>16.4?}   nodes: {:>10.2}   correct: {:<4}",
                name, strong_str, mean_time, mean_nodes, correct
            );
        }
    }
}

const SIZE: i32 = (Connect4::WIDTH * Connect4::HEIGHT) as i32;

fn solve(board: &Connect4, mut alpha: i32, mut beta: i32, depth: i32, nodes: &mut u32) -> i32 {
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

    let best_possible = (SIZE + 1 - depth) / 2;
    beta = min(beta, best_possible);
    if alpha >= beta {
        return beta;
    }

    for mv in [3, 2, 4, 1, 5, 0, 6] {
        if !board.is_available_move(mv) {
            continue;
        }

        let score = -solve(&board.clone_and_play(mv), -beta, -alpha, depth + 1, nodes);
        if score >= beta {
            return score;
        }

        alpha = max(alpha, score);
    }

    return alpha;
}

fn debug_board(board: &Connect4, depth: i32) {
    if !board.is_done() {
        board.available_moves().for_each(|mv| {
            let child_value = -solve(&board.clone_and_play(mv), -i32::MAX, i32::MAX, depth, &mut 0);
            println!("{} => {}", mv, child_value);
        })
    }
}