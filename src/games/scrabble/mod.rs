//! # Resources used:
//! ## Standards:
//! - https://users.cs.northwestern.edu/~robby/uc-courses/22001-2008-winter/scrabble.html
//! - https://www.poslfit.com/scrabble/gcg/
//!
//! ## Papers:
//! - [Appel, Jacobson (1988)](https://www.cs.cmu.edu/afs/cs/academic/class/15451-s06/www/lectures/scrabble.pdf)
//! - [Gordon (1993)](https://ericsink.com/downloads/faster-scrabble-gordon.pdf)
//!
//! ## Blogs
//! - https://cesardelsolar.com/posts/2023-06-14-scrabble-endgames-chess-techniques/
//! - https://medium.com/@14domino/scrabble-is-nowhere-close-to-a-solved-game-6628ec9f5ab0
//! - https://amedee.me/2020/11/04/fst-gaddag/
//! - http://www.breakingthegame.net/computerseries
//!
//! ## Engines
//! - https://github.com/domino14/macondo
//! - https://github.com/andy-k/wolges/

pub mod basic;
pub mod board;
pub mod grid;
pub mod movegen;
pub mod zobrist;
