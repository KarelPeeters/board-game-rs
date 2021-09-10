use std::fmt::{Display, Formatter};

use chess::{BoardStatus, ChessMove, Color, MoveGen};
use internal_iterator::{Internal, InternalIterator, IteratorExt};
use rand::Rng;

use crate::board::{Board, BoardAvailableMoves, Outcome, Player};
use crate::symmetry::UnitSymmetry;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    pub inner: chess::Board,
}

impl ChessBoard {
    pub fn new(inner: chess::Board) -> Self {
        ChessBoard { inner }
    }
}

impl Board for ChessBoard {
    type Move = chess::ChessMove;
    type Symmetry = UnitSymmetry;

    fn can_lose_after_move() -> bool {
        false
    }

    fn next_player(&self) -> Player {
        color_to_player(self.inner.side_to_move())
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        self.inner.legal(mv)
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        let mut move_gen = MoveGen::new_legal(&self.inner);
        let picked = rng.gen_range(0..move_gen.len());
        move_gen.nth(picked).unwrap()
    }

    fn play(&mut self, mv: Self::Move) {
        *self = self.clone_and_play(mv)
    }

    fn clone_and_play(&self, mv: Self::Move) -> Self {
        ChessBoard { inner: self.inner.make_move_new(mv) }
    }

    fn outcome(&self) -> Option<Outcome> {
        match self.inner.status() {
            BoardStatus::Ongoing => None,
            BoardStatus::Stalemate => Some(Outcome::Draw),
            BoardStatus::Checkmate => Some(Outcome::WonBy(self.next_player().other()))
        }
    }

    fn map(&self, _: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(_: Self::Symmetry, mv: Self::Move) -> Self::Move {
        mv
    }
}

#[derive(Debug)]
pub struct AllMoveIterator;

impl InternalIterator for AllMoveIterator {
    type Item = ChessMove;
    fn find_map<R, F>(self, mut f: F) -> Option<R> where F: FnMut(Self::Item) -> Option<R> {
        for from in chess::ALL_SQUARES {
            for to in chess::ALL_SQUARES {
                if let Some(x) = f(ChessMove::new(from, to, None)) {
                    return Some(x);
                }

                for piece in chess::PROMOTION_PIECES {
                    if let Some(x) = f(ChessMove::new(from, to, Some(piece))) {
                        return Some(x);
                    }
                }
            }
        }

        None
    }
}

impl<'a> BoardAvailableMoves<'a, ChessBoard> for ChessBoard {
    type MoveIterator = Internal<MoveGen>;
    type AllMoveIterator = AllMoveIterator;

    fn all_possible_moves() -> Self::AllMoveIterator {
        AllMoveIterator
    }

    fn available_moves(&'a self) -> Self::MoveIterator {
        MoveGen::new_legal(&self.inner).into_internal()
    }
}

fn color_to_player(color: chess::Color) -> Player {
    match color {
        Color::White => Player::A,
        Color::Black => Player::B,
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        ChessBoard {
            inner: chess::Board::default()
        }
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChessBoard(\"{}\")", self.inner)
    }
}
