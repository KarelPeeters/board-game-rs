//! Utilities for collecting game statistics and testing game and bot implementations.
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use internal_iterator::InternalIterator;

use crate::ai::Bot;
use crate::board::Board;

/// The number of legal positions reachable after `depth` moves, including duplicates.
/// See <https://www.chessprogramming.org/Perft>.
pub fn perft<B: Board>(board: &B, depth: u32) -> u64 {
    let mut map = HashMap::default();
    perft_recurse(&mut map, board.clone(), depth)
}

fn perft_recurse<B: Board + Hash>(map: &mut HashMap<(B, u32), u64>, board: B, depth: u32) -> u64 {
    //TODO we can move the counter one level up and count moves there, instead of actually playing them
    if depth == 0 {
        return 1;
    }
    if board.is_done() {
        return 0;
    }

    // we need keys (B, depth) because otherwise we risk miscounting if the same board is encountered at different depths
    let key = (board, depth);
    let board = &key.0;

    if let Some(&p) = map.get(&key) {
        return p;
    }

    let mut p = 0;
    board.available_moves().for_each(|mv: B::Move| {
        p += perft_recurse(map, board.clone_and_play(mv), depth - 1);
    });

    map.insert(key, p);
    p
}

/// Structure returned by [`average_game_stats`].
#[derive(Debug)]
pub struct GameStats {
    pub game_length: f32,
    pub available_moves: f32,
}

/// Return `GameStats` estimated from `n` games starting from `start` played by `bot`.
pub fn average_game_stats<B: Board>(start: &B, mut bot: impl Bot<B>, n: u64) -> GameStats {
    let mut total_moves = 0;
    let mut total_positions = 0;

    for _ in 0..n {
        let mut board = start.clone();
        while !board.is_done() {
            total_moves += board.available_moves().count();
            total_positions += 1;

            board.play(bot.select_move(&board));
        }
    }

    GameStats {
        game_length: total_positions as f32 / n as f32,
        available_moves: total_moves as f32 / total_positions as f32,
    }
}

/// Generate the set of all possible board positions reachable from the given board.
/// This function can easily take a long time to terminate or not terminate at all depending on the game.
pub fn all_possible_boards<B: Board>(start: &B, include_done: bool) -> Vec<B> {
    let mut set = HashSet::new();
    let mut result = vec![];
    all_possible_boards_impl(start, include_done, &mut result, &mut set);
    result
}

fn all_possible_boards_impl<B: Board>(start: &B, include_done: bool, result: &mut Vec<B>, set: &mut HashSet<B>) {
    if !include_done && start.is_done() {
        return;
    }
    if !set.insert(start.clone()) {
        return;
    }
    result.push(start.clone());
    if start.is_done() {
        return;
    }

    start
        .available_moves()
        .for_each(|mv| all_possible_boards_impl(&start.clone_and_play(mv), include_done, result, set))
}
