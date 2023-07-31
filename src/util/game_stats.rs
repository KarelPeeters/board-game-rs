//! Utilities for collecting game statistics and testing game and bot implementations.
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use internal_iterator::InternalIterator;
use rand::Rng;

use crate::ai::Bot;
use crate::board::{Board, Player};
use crate::pov::NonPov;
use crate::wdl::WDL;

/// The number of legal positions reachable after `depth` moves, including duplicates.
/// See <https://www.chessprogramming.org/Perft>.
pub fn perft<B: Board + Hash>(board: &B, depth: u32) -> u64 {
    let mut map = HashMap::default();
    perft_recurse(&mut map, board.clone(), depth)
}

fn perft_recurse<B: Board + Hash>(map: &mut HashMap<(B, u32), u64>, board: B, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    if board.is_done() {
        return 0;
    }
    if depth == 1 {
        return board.available_moves().unwrap().count() as u64;
    }

    // we need keys (B, depth) because otherwise we risk miscounting if the same board is encountered at different depths
    let key = (board, depth);
    let board = &key.0;

    if let Some(&p) = map.get(&key) {
        return p;
    }

    let mut p = 0;
    board.children().unwrap().for_each(|(_, child)| {
        p += perft_recurse(map, child, depth - 1);
    });

    map.insert(key, p);
    p
}

/// Same as [perft] but without any caching of perft values for visited boards.
pub fn perft_naive<B: Board>(board: &B, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    if board.is_done() {
        return 0;
    }
    if depth == 1 {
        return board.available_moves().unwrap().count() as u64;
    }

    let mut p = 0;
    board.available_moves().unwrap().for_each(|mv: B::Move| {
        p += perft_naive(&board.clone_and_play(mv).unwrap(), depth - 1);
    });
    p
}

/// Structure returned by [`average_game_stats`].
#[derive(Debug)]
pub struct GameStats {
    pub game_length: f32,
    pub available_moves: f32,
    pub total_wdl_a: WDL<u64>,
}

/// Return `GameStats` estimated from `n` games starting from `start` played by `bot`.
pub fn average_game_stats<B: Board>(mut start: impl FnMut() -> B, mut bot: impl Bot<B>, n: u64) -> GameStats {
    let mut total_moves = 0;
    let mut total_positions = 0;
    let mut total_wdl_a = WDL::default();

    for _ in 0..n {
        let mut board = start();

        let outcome = loop {
            total_moves += board.available_moves().unwrap().count();
            total_positions += 1;

            board.play(bot.select_move(&board).unwrap()).unwrap();

            if let Some(outcome) = board.outcome() {
                break outcome;
            }
        };

        total_wdl_a += outcome.pov(Player::A).to_wdl();
    }

    GameStats {
        game_length: total_positions as f32 / n as f32,
        available_moves: total_moves as f32 / total_positions as f32,
        total_wdl_a,
    }
}

/// Generate the set of all possible board positions reachable from the given board, in `depth` moves or less.
/// The returned vec does not contain duplicate elements.
///
/// **Warning**: This function can easily take a long time to terminate or not terminate at all depending on the game.
pub fn all_possible_boards<B: Board + Hash>(start: &B, depth: u32, include_done: bool) -> Vec<B> {
    let mut set = HashSet::new();
    let mut result = vec![];
    all_possible_boards_impl(start, depth, include_done, &mut result, &mut set);
    result
}

fn all_possible_boards_impl<B: Board + Hash>(
    start: &B,
    depth: u32,
    include_done: bool,
    result: &mut Vec<B>,
    set: &mut HashSet<B>,
) {
    if !include_done && start.is_done() {
        return;
    }
    if !set.insert(start.clone()) {
        return;
    }
    result.push(start.clone());
    if start.is_done() || depth == 0 {
        return;
    }

    start
        .children()
        .unwrap()
        .for_each(|(_, child)| all_possible_boards_impl(&child, depth - 1, include_done, result, set));
}

/// Collect all available moves form `n` games played until the end with random moves.
/// Also returns the number of time each move was availalbe.
pub fn all_available_moves_sampled<B: Board>(start: &B, n: u64, rng: &mut impl Rng) -> HashMap<B::Move, u64>
where
    B::Move: Hash,
{
    let mut moves = HashMap::default();

    for _ in 0..n {
        let mut curr = start.clone();
        while !curr.is_done() {
            curr.available_moves().unwrap().for_each(|mv| {
                *moves.entry(mv).or_default() += 1;
            });
            curr.play_random_available_move(rng).unwrap();
        }
    }

    moves
}
