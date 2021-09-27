use std::fmt::{Debug, Write};
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use chess::{BoardStatus, ChessMove, Color, MoveGen, Piece};
use internal_iterator::{Internal, InternalIterator, IteratorExt};
use rand::Rng;

use crate::board::{Board, BoardAvailableMoves, Outcome, Player};
use crate::symmetry::UnitSymmetry;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    max_repetitions: Option<u16>,
    max_moves_without_pawn_or_capture: Option<u16>,
    max_game_length: Option<u16>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    inner: chess::Board,
    rules: Rules,

    // 0 if this is the first time this position was reached
    //TODO remove pub and create getters
    pub repetitions: u16,
    pub non_pawn_or_capture_moves: u16,
    pub game_length: u16,

    history: Vec<u64>,
}

impl ChessBoard {
    pub fn default_with_rules(rules: Rules) -> Self {
        Self::new(chess::Board::default(), rules)
    }

    //TODO expose other fields in constructor again
    pub fn new(inner: chess::Board, rules: Rules) -> Self {
        ChessBoard {
            inner,
            rules,
            repetitions: 0,
            non_pawn_or_capture_moves: 0,
            game_length: 0,
            history: vec![],
        }
    }

    pub fn inner(&self) -> &chess::Board {
        &self.inner
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        ChessBoard::default_with_rules(Rules::default())
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
        move_gen.nth(picked).unwrap()
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(!self.is_done());
        *self = self.clone_and_play(mv)
    }

    fn clone_and_play(&self, mv: Self::Move) -> Self {
        assert!(!self.is_done());

        let new_inner = self.inner.make_move_new(mv);

        let side_to_move = self.inner.side_to_move();
        let moved_piece = self.inner.piece_on(mv.get_source());

        let capture = self.inner.color_on(mv.get_dest()).is_some();
        let pawn_move = moved_piece == Some(Piece::Pawn);
        let removed_en_passant = self.inner.en_passant().is_some();
        let removed_castle = self.inner.castle_rights(side_to_move) != new_inner.castle_rights(side_to_move);

        let new_non_pawn_or_capture_moves = if capture || pawn_move {
            0
        } else {
            self.non_pawn_or_capture_moves + 1
        };

        // reset history if any non-reversible move is made or there is no limit on repetitions
        let reset_history =
            capture || pawn_move || removed_en_passant || removed_castle || self.rules.max_repetitions.is_none();
        let new_history = if reset_history {
            vec![]
        } else {
            let mut new_history = self.history.clone();
            new_history.push(self.inner.get_hash());
            new_history
        };

        let repetitions = new_history.iter().filter(|&&h| new_inner.get_hash() == h).count() as u16;

        ChessBoard {
            inner: new_inner,
            rules: self.rules,
            repetitions,
            non_pawn_or_capture_moves: new_non_pawn_or_capture_moves,
            game_length: self.game_length + 1,
            history: new_history,
        }
    }

    fn outcome(&self) -> Option<Outcome> {
        if self
            .rules
            .is_draw(self.repetitions, self.non_pawn_or_capture_moves, self.game_length)
        {
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

pub fn color_to_player(color: Color) -> Player {
    match color {
        Color::White => Player::A,
        Color::Black => Player::B,
    }
}

pub fn player_to_color(player: Player) -> Color {
    match player {
        Player::A => Color::White,
        Player::B => Color::Black,
    }
}

impl Rules {
    pub fn ccrl() -> Self {
        Rules {
            max_repetitions: None,
            max_moves_without_pawn_or_capture: Some(100),
            max_game_length: None,
        }
    }

    pub fn unlimited() -> Self {
        Rules {
            max_repetitions: None,
            max_moves_without_pawn_or_capture: None,
            max_game_length: None,
        }
    }

    pub fn is_draw(self, repetitions: u16, non_pawn_or_capture_moves: u16, game_length: u16) -> bool {
        self.max_repetitions.map_or(false, |m| repetitions > m)
            || self
                .max_moves_without_pawn_or_capture
                .map_or(false, |m| non_pawn_or_capture_moves > m)
            || self.max_game_length.map_or(false, |m| game_length > m)
    }
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            max_repetitions: Some(3),
            max_moves_without_pawn_or_capture: Some(100),
            max_game_length: None,
        }
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChessBoard {{ inner: \"{}\", game_length: {}, non_pawn_or_capture_moves: {}, repetitions: {}, rules: {:?}, history: {:?} }}",
            self.inner, self.game_length, self.non_pawn_or_capture_moves, self.repetitions, self.rules, self.history
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
