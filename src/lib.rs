#![warn(missing_debug_implementations)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unusual_byte_groupings)]
#![allow(clippy::derived_hash_with_manual_eq)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::new_without_default)]

//! A [Board](crate::board::Board) abstraction for deterministic two player games.
//! This allows for code to be generic over the actual game, so it only needs to written once.
//!
//! # Features
//!
//! Currently, the implemented games are:
//! * [Chess](https://en.wikipedia.org/wiki/Chess) as [ChessBoard](crate::games::chess::ChessBoard),
//!     implemented as a simple wrapper around the [chess](https://crates.io/crates/chess) crate.
//! * [Go/Baduk](https://en.wikipedia.org/wiki/Go_(game))
//!     as [GoBoard](crate::games::go::board::GoBoard).
//! * [Super/Ultimate tic-tac-toe](https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe)
//!     as [STTTBoard](crate::games::sttt::STTTBoard).
//! * [Ataxx](https://en.wikipedia.org/wiki/Ataxx)
//!     as [AtaxxBoard](crate::games::ataxx::board::AtaxxBoard).
//! * [Oware](https://en.wikipedia.org/wiki/Oware) as [OwareBoard](crate::games::oware::OwareBoard).
//! * [Connect4](https://en.wikipedia.org/wiki/Connect_Four) as [Connect4](crate::games::connect4::Connect4).
//! * [Tic Tac Toe](https://en.wikipedia.org/wiki/Tic-tac-toe) as [TTTBoard](crate::games::ttt::TTTBoard).
//!
//! Most game implementations are heavily optimized, using bitboards or other techniques where appropriate.
//!
//! There are also some utility boards:
//! * [MaxMovesBoard](crate::games::max_length::MaxMovesBoard)
//!     wraps another board and sets the outcome to a draw after move limit has been reached.
//! * [DummyGame](crate::games::dummy::DummyGame)
//!     is a board that is constructed from an explicit game tree, useful for debugging.
//!
//! Utilities in this crate that work for any [Board](crate::board::Board):
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
//! * Simple game statistics (perft, random game length) which can be used to test board implementations.
//!
//! This crate is also used as the foundation for [kZero](https://github.com/KarelPeeters/kZero),
//! a general AlphaZero implementation.
//!
//! # Examples
//!
//! ## List the available moves on a board and play a random one.
//!
//! ```
//! # #[cfg(feature = "game_ataxx")]
//! # use board_game::games::ataxx::AtaxxBoard;
//! # use board_game::board::{BoardMoves, Board};
//! # use internal_iterator::InternalIterator;
//! # let mut rng = rand::thread_rng();
//! # #[cfg(feature = "game_ataxx")]
//! # {
//! let mut board = AtaxxBoard::default();
//! println!("{}", board);
//!
//! board.available_moves().unwrap().for_each(|mv| {
//!     println!("{:?}", mv)
//! });
//!
//! let mv = board.random_available_move(&mut rng).unwrap();
//! println!("Picked move {:?}", mv);
//! board.play(mv).unwrap();
//! println!("{}", board);
//! # }
//! ```
//!
//! ## Get the best move according to MCTS
//!
//! ```
//! # use board_game::ai::mcts::MCTSBot;
//! # #[cfg(feature = "game_ataxx")]
//! # use board_game::games::ataxx::AtaxxBoard;
//! # use board_game::ai::Bot;
//! # use rand::thread_rng;
//! # #[cfg(feature = "game_ataxx")]
//! # {
//! let board = AtaxxBoard::default();
//! println!("{}", board);
//!
//! let mut bot = MCTSBot::new(1000, 2.0, thread_rng());
//! println!("{:?}", bot.select_move(&board))
//! # }
//! ```

// export used game crates

#[cfg(feature = "game_arimaa")]
pub use arimaa_engine_step;

#[cfg(feature = "game_chess")]
pub use chess;

pub mod board;
pub mod symmetry;

pub mod pov;
pub mod wdl;

pub mod ai;

pub mod games;

pub mod heuristic;

pub mod util;

pub mod interface;
