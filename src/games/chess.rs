use std::fmt::Write;
use std::fmt::{Display, Formatter};

use chess::{BoardStatus, ChessMove, Color, MoveGen, Piece};
use internal_iterator::{Internal, InternalIterator, IteratorExt};
use rand::Rng;

use crate::board::{Board, BoardAvailableMoves, Outcome, Player};
use crate::symmetry::UnitSymmetry;

pub const MAX_REVERSIBLE_MOVES: u32 = 100;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    inner: chess::Board,
    /// The number of consecutive reversible moves, resets when an irreversible move is played.
    reversible_moves: u32,
}

impl ChessBoard {
    pub fn new(inner: chess::Board, reversible_moves: u32) -> Self {
        ChessBoard {
            inner,
            reversible_moves,
        }
    }

    pub fn inner(&self) -> &chess::Board {
        &self.inner
    }
    pub fn reversible_moves(&self) -> u32 {
        self.reversible_moves
    }
}

impl Board for ChessBoard {
    type Move = ChessMove;
    type Symmetry = UnitSymmetry;

    fn can_lose_after_move() -> bool {
        false
    }

    fn next_player(&self) -> Player {
        color_to_player(self.inner.side_to_move())
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());
        self.inner.legal(mv)
    }

    fn random_available_move(&self, rng: &mut impl Rng) -> Self::Move {
        assert!(!self.is_done());
        let mut move_gen = MoveGen::new_legal(&self.inner);
        let picked = rng.gen_range(0..move_gen.len());
        // SAFETY: unwrap is safe because the index is less than the
        // number of objects in the iterator.
        move_gen.nth(picked).unwrap()
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(!self.is_done());
        *self = self.clone_and_play(mv)
    }

    fn clone_and_play(&self, mv: Self::Move) -> Self {
        assert!(!self.is_done());

        let capture = self.inner.color_on(mv.get_dest()).is_some();
        let pawn_move = self.inner.piece_on(mv.get_source()) == Some(Piece::Pawn);
        let reversible_moves = if capture || pawn_move {
            0
        } else {
            self.reversible_moves + 1
        };

        ChessBoard {
            inner: self.inner.make_move_new(mv),
            reversible_moves,
        }
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.reversible_moves >= MAX_REVERSIBLE_MOVES {
            Some(Outcome::Draw)
        } else {
            match self.inner.status() {
                BoardStatus::Ongoing => None,
                BoardStatus::Stalemate => Some(Outcome::Draw),
                BoardStatus::Checkmate => Some(Outcome::WonBy(self.next_player().other())),
            }
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
    fn find_map<R, F>(self, mut f: F) -> Option<R>
    where
        F: FnMut(Self::Item) -> Option<R>,
    {
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
        assert!(!self.is_done());
        MoveGen::new_legal(&self.inner).into_internal()
    }
}

fn color_to_player(color: Color) -> Player {
    match color {
        Color::White => Player::A,
        Color::Black => Player::B,
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        ChessBoard {
            inner: chess::Board::default(),
            reversible_moves: 0,
        }
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChessBoard(\"{}\", reversible_moves: {})",
            self.inner, self.reversible_moves
        )
    }
}

pub fn moves_to_pgn(moves: &[ChessMove]) -> String {
    let mut result = String::new();
    let f = &mut result;

    for (i, mv) in moves.iter().enumerate() {
        if i % 2 == 0 {
            write!(f, "{}. ", 1 + i / 2).unwrap();
        }

        write!(f, "{}{}", mv.get_source(), mv.get_dest()).unwrap();
        if let Some(promotion) = mv.get_promotion() {
            write!(f, "={}", promotion.to_string(Color::White)).unwrap()
        };

        write!(f, " ").unwrap();
    }

    result
}
