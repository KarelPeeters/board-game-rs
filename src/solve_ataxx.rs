use std::collections::hash_map::Entry;
use std::time::Instant;

use fnv::FnvHashMap;
use internal_iterator::{InternalIterator, IteratorExt};

use crate::board::{Board, BoardAvailableMoves, Player};
use crate::games::ataxx::{AtaxxBoard, Tiles};
use crate::symmetry::Symmetry;
use crate::wdl::{Flip, OutcomeWDL, POV};

//TODO extra recently used cache for better cache coherency?

pub fn main() {
    let board = AtaxxBoard::diagonal(5);
    println!("{}", board);

    let mut storage = Storage::default();

    let start = Instant::now();

    let s = solve_ataxx(&board, &mut storage, 0);

    // println!("Storage:");
    // for (k, v) in &storage {
    //     let s = format!("{:?}", k.to(board.size()));
    //     let p = " ".repeat(40 - s.len());
    //     println!("  {}{}: {:?}", s, p, v);
    // }

    println!("storage size: {}", storage.len());
    println!("solution:     {:?}", s);
    println!("took:         {:?}", start.elapsed());

    board.available_moves().for_each(|mv| {
        let s2 = solve_ataxx(&board.clone_and_play(mv), &mut storage, 1);
        println!("{}: {:?}", mv, s2);
    })
}

type Storage = FnvHashMap<ReducedBoard, Info>;

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
pub struct Info {
    outcome: Option<OutcomeWDL>,
}

fn solve_ataxx(board_original: &AtaxxBoard, storage: &mut Storage, depth: u32) -> OutcomeWDL {
    if let Some(outcome) = board_original.outcome() {
        return outcome.pov(board_original.next_player());
    }

    let board_reduced = ReducedBoard::from(board_original);

    match storage.entry(board_reduced) {
        Entry::Occupied(mut entry) => {
            let info = entry.get_mut();
            return if let Some(outcome) = info.outcome {
                // standard cache hit
                outcome
            } else {
                // cycles are considered draws
                OutcomeWDL::Draw
            };
        }
        Entry::Vacant(entry) => {
            // mark this entry as part of the current game to later detect draws
            //   the outcome will be filled in later
            entry.insert(Info { outcome: None });
        }
    }

    let board_continue = board_reduced.to(board_original.size);
    let mut next_boards: Vec<_> = board_continue
        .available_moves()
        .map(|mv| board_continue.clone_and_play(mv))
        .collect();

    // order from low to high by opponent tiles
    next_boards.sort_by_key(|b| b.tiles_pov().0.count());

    // recurse with minimax
    let outcome = OutcomeWDL::best(
        next_boards
            .iter()
            .map(|b| solve_ataxx(b, storage, depth + 1).flip())
            .into_internal(),
    );

    // finally fill in the outcome we left empty earlier
    let info = storage.get_mut(&board_reduced).unwrap();
    assert!(info.outcome.is_none());
    info.outcome = Some(outcome);

    outcome
}
