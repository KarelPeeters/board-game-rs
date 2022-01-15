use std::time::Instant;

use fnv::FnvHashMap;
use internal_iterator::{InternalIterator, IteratorExt};

use board_game::board::{Board, BoardAvailableMoves};
use board_game::games::ataxx::{AtaxxBoard, Tiles};
use board_game::symmetry::Symmetry;
use board_game::wdl::{Flip, OutcomeWDL, POV};

//TODO extra recently used cache for better cache coherency?
//TODO transpose into older boards if win?

fn main() {
    let board = AtaxxBoard::diagonal(5);
    println!("{}", board);

    let mut cache = OutcomeCache::default();

    let start = Instant::now();

    for depth in 0.. {
        println!("Depth: {}", depth);

        println!("  cache before: {}", cache.len());
        cache.retain(|_, v| v.is_some());
        println!("  cache after:  {}", cache.len());

        let s = solve_ataxx(&board, &mut cache, depth);
        println!("  result: {:?}", s);
        println!("  time: {:?}", start.elapsed());

        if s.is_some() {
            break;
        }
    }
}

#[repr(packed)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct ReducedBoard {
    tiles_next: Tiles,
    tiles_other: Tiles,
    moves_since_last_copy: u8,
}

type OutcomeCache = FnvHashMap<ReducedBoard, Option<OutcomeWDL>>;

fn solve_ataxx(board: &AtaxxBoard, cache: &mut OutcomeCache, depth: u32) -> Option<OutcomeWDL> {
    assert!(board.gaps().is_empty());

    if let Some(outcome) = board.outcome() {
        return Some(outcome.pov(board.next_player()));
    }

    if depth == 0 {
        return None;
    }

    // canonical lookup key
    let key = {
        let size = board.size();
        let (tiles_next, tiles_other) = board.tiles_pov();

        let sym = <AtaxxBoard as Board>::Symmetry::all()
            .iter()
            .copied()
            .min_by_key(|&s| tiles_next.map(size, s).inner())
            .unwrap();

        ReducedBoard {
            tiles_next: tiles_next.map(size, sym),
            tiles_other: tiles_other.map(size, sym),
            moves_since_last_copy: board.moves_since_last_copy(),
        }
    };

    if let Some(&outcome) = cache.get(&key) {
        return outcome;
    }

    let mut next_boards: Vec<_> = board.available_moves().map(|mv| board.clone_and_play(mv)).collect();
    //TODO why is this so counterintuitive?
    next_boards.sort_by_key(|b| b.tiles_pov().1.count());

    let result = OutcomeWDL::best(
        next_boards
            .iter()
            .map(|b| solve_ataxx(b, cache, depth - 1).flip())
            .into_internal(),
    );

    cache.insert(key, result);

    result
}
