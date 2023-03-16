//! Dummy game useful for debugging purposes.
//!
//! It is simply a tree that leads to different outcomes.
//!
//! # Example
//!
//! ```
//! use board_game::games::dummy::DummyGame;
//! use board_game::board::{Board, Outcome, Player};
//!
//! let game: DummyGame = "A".parse().unwrap();
//! assert_eq!(game.outcome(), Some(Outcome::WonBy(Player::A)));
//! let game: DummyGame = "B".parse().unwrap();
//! assert_eq!(game.outcome(), Some(Outcome::WonBy(Player::B)));
//! let game: DummyGame = "=".parse().unwrap();
//! assert_eq!(game.outcome(), Some(Outcome::Draw));
//!
//! let game: DummyGame = "(AA(BB)=B)".parse().unwrap();
//! // This board has 5 moves:
//! // * the first two lead to a victory by A
//! // * the third one leads to a board with two moves: both victories by B
//! // * the fourth move leads to a draw
//! // * the fifth move leads to a victory by B
//! ```
use std::fmt;
use std::str::FromStr;

use internal_iterator::{Internal, IteratorExt};
use nom::error::Error;
use nom::Finish;

use crate::board::{Alternating, Board, BoardDone, BoardMoves, Outcome, PlayError, Player};
use crate::impl_unit_symmetry_board;

mod parse {
    use nom::branch::alt;
    use nom::character::complete::{char, one_of};
    use nom::combinator::{eof, map};
    use nom::multi::many1;
    use nom::sequence::{delimited, terminated};
    use nom::IResult;

    use super::*;

    fn outcome(input: &str) -> IResult<&str, Outcome> {
        map(one_of("AB="), |c| match c {
            'A' => Outcome::WonBy(Player::A),
            'B' => Outcome::WonBy(Player::B),
            '=' => Outcome::Draw,
            _ => unreachable!(),
        })(input)
    }

    fn node(input: &str) -> IResult<&str, Tree> {
        alt((
            map(outcome, Tree::Outcome),
            map(delimited(char('('), many1(node), char(')')), |games: Vec<Tree>| {
                Tree::Node(games)
            }),
        ))(input)
    }

    pub(super) fn tree(input: &str) -> IResult<&str, Tree> {
        terminated(node, eof)(input)
    }
}

impl FromStr for Tree {
    type Err = Error<String>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match parse::tree(s).finish() {
            Ok((_, tree)) => Ok(tree),
            Err(Error { input, code }) => Err(Error {
                input: input.to_string(),
                code,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Tree {
    Outcome(Outcome),
    Node(Vec<Tree>),
}

impl Tree {
    fn choose(&mut self, i: usize) {
        if let Tree::Node(boards) = self {
            *self = boards.swap_remove(i);
        } else {
            panic!()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DummyGame {
    state: Tree,
    player: Player,
}

impl FromStr for DummyGame {
    type Err = Error<String>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DummyGame {
            state: s.parse()?,
            player: Player::A,
        })
    }
}

impl fmt::Display for DummyGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Board for DummyGame {
    type Move = usize;

    fn next_player(&self) -> Player {
        self.player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        if let Tree::Node(boards) = &self.state {
            Ok(mv < boards.len())
        } else {
            Ok(false)
        }
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_can_play(mv)?;
        self.state.choose(mv);
        self.player = self.player.other();
        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        match self.state {
            Tree::Node(_) => None,
            Tree::Outcome(outcome) => Some(outcome),
        }
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

// TODO maybe add a version of DummyGame that's not alternating?
impl Alternating for DummyGame {}

impl_unit_symmetry_board!(DummyGame);

impl<'a> BoardMoves<'a, DummyGame> for DummyGame {
    type AllMovesIterator = Internal<std::ops::RangeFrom<usize>>;
    type AvailableMovesIterator = Internal<std::ops::Range<usize>>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        //TODO questionable, maybe we could take &self here and base us on that?
        (0..).into_internal()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        if let Tree::Node(boards) = &self.state {
            Ok((0..boards.len()).into_internal())
        } else {
            Err(BoardDone)
        }
    }
}
