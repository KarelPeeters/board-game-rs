#![allow(dead_code)]

use board_game::ai::minimax::minimax;
use internal_iterator::InternalIterator;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

use board_game::ai::solver::{solve, solve_all_moves};
use board_game::board::{Board, BoardMoves};
use board_game::games::ataxx::{AtaxxBoard, Move, PrevTerminal};
use board_game::heuristic::ataxx::AtaxxTileHeuristic;
use board_game::util::game_stats::perft;

fn main() {
    // demo();
    // fuzz();
    // tree();
    // custom();
    derp();
}

fn derp() {
    // let board = AtaxxBoard::from_fen("xxxxo/ooxoo/ooooo/xxxxx/xxx2 x 0 1").unwrap();
    let mut board = AtaxxBoard::from_fen("ooo1o/1oxxx/xxxxx/xxxxx/ooooo o 0 1").unwrap();
    println!("{}", board);

    let mut seen = HashSet::new();

    loop {
        if !seen.insert(board.clone()) {
            break;
        }

        for depth in 0.. {
            let eval = solve_all_moves(&board, depth);
            // let eval = minimax(&board, &AtaxxTileHeuristic::default(), depth, &mut rng);
            // println!("depth {}: {:?}", depth, eval);

            // if eval.value.to_outcome_wdl().is_some() {
            //     break;
            // }

            if depth >= 20 {
                let mv = eval.best_move.unwrap()[0];
                println!("Playing {}", mv);
                board.play(mv).unwrap();
                board.clear_moves_since_last_copy();
                println!("{}", board);
                break;
            }
        }
    }
}

fn custom() {
    let mut board = AtaxxBoard::from_fen("7/7/7/7/7/7/xx5 o 0 1").unwrap();
    println!("{}", board);

    let mut count = 0;

    board.back_moves().for_each(|back| {
        let mut prev = board.clone();
        if prev.play_back(back).is_ok() {
            println!("{:?}", back);
            println!("{}", prev);
            count += 1;
        }
    });

    println!("{}", count);
}

fn tree() {
    let board = AtaxxBoard::from_fen("-------/-------/x1x2--/1x3--/1x3--/5--/4x-- o 0 1").unwrap();
    println!("board:\n{}", board);

    board.back_moves().for_each(|back_0| {
        let mut prev_0 = board.clone();
        if prev_0.play_back(back_0).is_err() {
            println!("{:?} => terminal", back_0);
            return;
        } else {
            println!("{:?} => {:?}", back_0, prev_0);
        }

        prev_0.back_moves().for_each(|back_1| {
            let mut prev_1 = prev_0.clone();
            if prev_1.play_back(back_1).is_err() {
                println!("  {:?} => terminal", back_1);
                return;
            } else {
                println!("  {:?} => {:?}", back_1, prev_1);
            }
        })
    })
}

#[derive(Default)]
struct Count {
    pos: u64,
    back: u64,
    mv: u64,
}

fn fuzz() {
    // deterministic rng
    let mut rng = SmallRng::seed_from_u64(0);

    let mut count = Count::default();

    fuzz_test(&AtaxxBoard::diagonal(7), &mut count);

    for _ in 0..1000 {
        let mut board = AtaxxBoard::diagonal(rng.gen_range(2..=8));
        fuzz_test(&board, &mut count);

        while let Ok(()) = board.play_random_available_move(&mut rng) {
            fuzz_test(&board, &mut count);
        }
    }

    println!("Visited {} random positions", count.pos);
    println!("  {} mv/pos", count.mv as f64 / count.pos as f64);
    println!("  {} back/pos", count.back as f64 / count.pos as f64);
}

fn fuzz_test(board: &AtaxxBoard, count: &mut Count) {
    println!("board {:?}", board);
    // println!("{}", board);

    count.pos += 1;
    count.mv += board.available_moves().map_or(0, |a| a.count() as u64);
    count.back += board.back_moves().count() as u64;

    // clear the move counter
    let mut board_clear = board.clone();
    board_clear.clear_moves_since_last_copy();

    // check that all back moves are valid and distinct
    let mut all_back = vec![];
    let mut all_prev = vec![];

    board.back_moves().for_each(|back| {
        // println!("  checking back {:?}", back);

        // ensure the back move yields a valid board
        let mut prev = board.clone();
        let r = prev.play_back(back);
        match r {
            Ok(()) => {}
            Err(PrevTerminal) => return,
        }
        board.assert_valid();

        // ensure playing the corresponding move again works and yields original board
        let mut next = prev.clone_and_play(back.mv).unwrap();
        next.clear_moves_since_last_copy();
        assert_eq!(board_clear, next);

        // ensure the back move is unique
        assert!(!all_back.contains(&back));
        all_back.push(back);

        // ensure the resulting previous board is unique
        assert!(!all_prev.contains(&prev));
        all_prev.push(prev);
    });

    if board.is_done() {
        return;
    }

    board.available_moves().unwrap().for_each(|mv| {
        // println!("  playing move {}", mv);
        let child = board.clone_and_play(mv).unwrap();

        // println!("    child: {:?}", child);

        // get potential back moves
        let back_moves = child.back_moves().filter(|back| back.mv == mv).collect::<Vec<_>>();
        assert!(!back_moves.is_empty());

        // ensure the prev board matches exactly one back move
        let mut matched_count = 0;

        for back in back_moves {
            // println!("    back cand: {:?}", back);
            let mut prev = child.clone();
            let r = prev.play_back(back);

            match r {
                Ok(()) => {
                    // println!("      prev: {:?}", prev);
                    if prev == board_clear {
                        // println!("      matches!");
                        matched_count += 1;
                    }
                }
                Err(PrevTerminal) => continue,
            }
        }

        assert_eq!(matched_count, 1);
    });
}

fn _demo() {
    // let board = AtaxxBoard::diagonal(7);
    let board = AtaxxBoard::from_fen("x5o/7/7/7/7/oo5/oo4x x 0 1").unwrap();

    println!("{}", board);

    println!("Back moves:");
    board.back_moves().for_each(|mv| {
        println!("  {:?}", mv);
    })
}
