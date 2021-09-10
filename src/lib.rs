#![warn(missing_debug_implementations)]

//! A [Board](crate::board::Board) abstraction for deterministic two player games.
//! This code to be generic over the actual game, so it only needs to written once.
//!
//! # Features
//!
//! Currently, the implemented games are:
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
//!
//! # Examples
//!
//! ## List the available moves on a board and play a random one.
//!
//! ```
//! # use board_game::games::ataxx::AtaxxBoard;
//! # use board_game::board::{BoardAvailableMoves, Board};
//! # use internal_iterator::InternalIterator;
//! # let mut rng = rand::thread_rng();
//!
//! let mut board = AtaxxBoard::default();
//! println!("{}", board);
//!
//! board.available_moves().for_each(|mv| {
//!     println!("{:?}", mv)
//! });
//!
//! let mv = board.random_available_move(&mut rng);
//! println!("Picked move {:?}", mv);
//! board.play(mv);
//! println!("{}", board);
//! ```
//!
//! ## Get the best move according to MCTS
//!
//! ```
//! # use board_game::ai::mcts::MCTSBot;
//! # use board_game::games::ataxx::AtaxxBoard;
//! # use board_game::ai::Bot;
//! # use rand::thread_rng;
//!
//! let board = AtaxxBoard::default();
//! println!("{}", board);
//!
//! let mut bot = MCTSBot::new(1000, 2.0, thread_rng());
//! println!("{:?}", bot.select_move(&board))
//! ```

pub mod board;
pub mod symmetry;
pub mod wdl;

pub mod games;
pub mod ai;
pub mod heuristic;

pub mod util;

pub mod uai;

