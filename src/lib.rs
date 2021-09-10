#![warn(missing_debug_implementations)]

//! A [Board](crate::board::Board) abstraction for deterministic two player games.
//! This code to be generic over the actual game, so it only needs to written once.
//!
//! Currently the implemented games are:
//! * [Super/Ultimate tic-tac-toe](https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe) in the module [sttt](crate::games::sttt).
//! * [Ataxx](https://en.wikipedia.org/wiki/Ataxx) in the module [ataxx](crate::games::ataxx).
//!
//! Notable things currently implemented in this crate that work for any [Board](crate::board::Board):
//! * Game-playing algorithms, specifically:
//!     * [RandomBot](crate::ai::simple::RandomBot),
//!         which simply picks a random move.
//!     * [RolloutBot](crate::ai::simple::RolloutBot),
//!         which simulates a fixed number of random games for each possible move and picks the one with the best win probability.
//!     * [MinimaxBot](crate::ai::minimax::MiniMaxBot),
//!         which picks the best move as evaluated by a customizable heuristic at a fixed depth. (implemented as alpha-beta negamax).
//!     * [MCTSBot](crate::ai::mcts::MCTSBot),
//!         which picks the best move as found by [Monte Carlo Tree Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search).
//! * Random board generation functions, see [board_gen](crate::util::board_gen).
//! * A bot vs bot game runner to compare playing strength, see [bot_game](crate::util::bot_game).
//! * Simple game statistics (perft, random game length) which can be used to test [Board](crate::board::Board) implementations.

pub mod board;
pub mod symmetry;
pub mod wdl;

pub mod games;
pub mod ai;
pub mod heuristic;

pub mod util;

pub mod uai;

