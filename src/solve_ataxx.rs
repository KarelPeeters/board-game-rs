#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use std::cmp::min;
use std::collections::hash_map::Entry;
use std::time::Instant;

use fnv::FnvHashMap;
use internal_iterator::{InternalIterator, IteratorExt};
use rand::prelude::SliceRandom;
use rand::thread_rng;

use crate::board::{Board, BoardAvailableMoves, Player};
use crate::games::ataxx::{AtaxxBoard, Tiles};
use crate::symmetry::Symmetry;
use crate::wdl::{Flip, OutcomeWDL, POV};

//TODO extra recently used cache for better cache coherency?

pub fn main() {
    // let board = AtaxxBoard::diagonal(5);
    let board = AtaxxBoard::diagonal(4);

    println!("{}", board);

    let start = Instant::now();
    let mut storage = Storage::default();

    let mut sort = |v: &mut Vec<AtaxxBoard>| v.sort_by_key(|b| b.tiles_pov().0.count());
    // let mut rng = thread_rng();
    // let mut sort = |v: &mut Vec<AtaxxBoard>| v.shuffle(&mut rng);

    // check_solver(&board, &mut storage, &mut sort, &start);
    // return;

    let s = solve_ataxx(&board, &mut storage, &mut sort, 0, 100, &start);

    println!("storage size: {}", storage.len());
    println!("solution:     {:?}", s);
    println!("took:         {:?}", start.elapsed());

    println!();
    board.available_moves().for_each(|mv| {
        let s2 = solve_ataxx(&board.clone_and_play(mv), &mut storage, &mut sort, 1, 100, &start);
        println!("{}: {:?}", mv, s2);
    });

    println!("storage size: {}", storage.len());
    println!("took:         {:?}", start.elapsed());
}

fn print_storage(size: u8, storage: &Storage) {
    println!("Storage:");
    for (k, v) in storage {
        let s = format!("{:?}", k.to(size));
        let p = " ".repeat(40 - s.len());
        println!("  {}{}: {:?}", s, p, v);
    }
}

// fn check_solver(
//     board: &AtaxxBoard,
//     storage: &mut Storage,
//     sort: &mut impl FnMut(&mut Vec<AtaxxBoard>) -> (),
//     start: &Instant,
// ) {
//     if board.is_done() {
//         return;
//     }
//
//     let board = ReducedBoard::from(board).to(board.size());
//     let expected = solve_ataxx(&board, storage, sort, 0, 0, start);
//
//     let actual = OutcomeWDL::best(
//         board
//             .available_moves()
//             .map(|mv| solve_ataxx(&board.clone_and_play(mv), storage, sort, 0, 0, start).flip()),
//     );
//
//     assert_eq!(expected, actual);
//
//     board.available_moves().for_each(|mv| {
//         check_solver(&board.clone_and_play(mv), storage, sort, start);
//     })
// }

type Storage = FnvHashMap<ReducedBoard, Info>;

//TODO maybe put both tiles in a single u64: 25 + 25 < 64
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct ReducedBoard {
    tiles_next: Tiles,
    tiles_other: Tiles,
}

impl ReducedBoard {
    fn from(board: &AtaxxBoard) -> Self {
        assert!(!board.is_done());
        assert!(board.gaps().is_empty());

        let size = board.size();
        let (tiles_next, tiles_other) = board.tiles_pov();

        // canonical reduction
        let sym = <AtaxxBoard as Board>::Symmetry::all()
            .iter()
            .copied()
            .min_by_key(|&s| tiles_next.map(size, s).inner())
            .unwrap();

        ReducedBoard {
            tiles_next: tiles_next.map(size, sym),
            tiles_other: tiles_other.map(size, sym),
        }
    }

    fn to(&self, size: u8) -> AtaxxBoard {
        AtaxxBoard {
            size,
            tiles_a: self.tiles_next,
            tiles_b: self.tiles_other,
            gaps: Tiles::empty(),
            moves_since_last_copy: 0,
            next_player: Player::A,
            outcome: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Info {
    eval: Eval,
}

#[derive(Debug, Copy, Clone)]
enum Eval {
    Outcome(OutcomeWDL),
    Cycle(u32),
}

fn solve_ataxx(
    board_original: &AtaxxBoard,
    storage: &mut Storage,
    sort: &mut impl FnMut(&mut Vec<AtaxxBoard>) -> (),
    depth: u32,
    print_depth: u32,
    start: &Instant,
) -> Eval {
    if depth < print_depth {
        print_curr_depth(depth, print_depth, start);
    }

    //TODO move these lines down
    let board_reduced = ReducedBoard::from(board_original);
    let board_continue = board_reduced.to(board_original.size);

    if let Some(outcome) = board_original.outcome() {
        let outcome = outcome.pov(board_original.next_player());
        println!("depth {} {:?} T {:?}", depth, board_original, outcome);
        return Eval::Outcome(outcome);
    }

    match storage.entry(board_reduced) {
        Entry::Occupied(entry) => {
            // just return the existing eval, works for both outcomes and cycles
            return entry.get().eval;
        }
        Entry::Vacant(entry) => {
            // mark this entry as part of the current game to later detect draws
            //   the outcome will be filled in later
            entry.insert(Info {
                eval: Eval::Cycle(depth),
            });
        }
    }

    let mut next_boards: Vec<_> = board_continue
        .available_moves()
        .map(|mv| board_continue.clone_and_play(mv))
        .collect();

    // order from low to high by opponent tiles
    // TODO why does subtracting our own tiles not improve things?
    // TODO bug: move ordering should not affect computed results!
    sort(&mut next_boards);

    // recurse with minimax
    let mut min_cycle_depth = u32::MAX;

    let outcome = OutcomeWDL::best(
        next_boards
            .iter()
            .map(|b| {
                let eval = solve_ataxx(b, storage, sort, depth + 1, print_depth, start);
                match eval {
                    Eval::Outcome(outcome) => outcome.flip(),
                    Eval::Cycle(depth) => {
                        min_cycle_depth = min(min_cycle_depth, depth);
                        OutcomeWDL::Draw
                    }
                }
            })
            .into_internal(),
    );

    if outcome == OutcomeWDL::Draw && (min_cycle_depth != u32::MAX && min_cycle_depth != depth) {
        // this node may not actually be a draw, so we can't fill it in
        storage.remove(&board_reduced);
        Eval::Cycle(min_cycle_depth)
    } else {
        // we fully know the true outcome of this node
        let eval = Eval::Outcome(outcome);
        storage.get_mut(&board_reduced).unwrap().eval = eval;
        eval
    }
}

fn print_curr_depth(depth: u32, max_depth: u32, start: &Instant) {
    for _ in 0..depth {
        print!(" ");
    }
    print!("{}", depth);
    for _ in depth..max_depth {
        print!(" ");
    }
    println!("{:?}", start.elapsed());
}
