#![allow(dead_code)]

use internal_iterator::InternalIterator;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use board_game::board::{Board, BoardMoves};
use board_game::games::ataxx::{AtaxxBoard, BackMove, Move, PrevTerminal};
use board_game::util::bitboard::BitBoard8;

fn main() {
    // demo();
    // fuzz();
    custom()
}

fn custom() {
    let board = AtaxxBoard::from_fen("x1x2/1x3/1x3/5/4x o 0 1").unwrap();
    println!("board:\n{}", board);

    board.back_moves().for_each(|back| {
        println!("  {:?}", back);
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
    if board.is_done() {
        return;
    }

    println!("board {:?}", board);
    // println!("{}", board);

    count.pos += 1;
    count.mv += board.available_moves().unwrap().count() as u64;
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
        prev.play_back(back).unwrap();
        board.assert_valid();

        // ensure playing the corresponding move again works and yields the same board
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
