use std::borrow::Cow;
use std::fmt::{Debug, Write};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::ControlFlow;
use std::str::FromStr;

use chess::{BoardStatus, ChessMove, Color, File, MoveGen, Piece};
use internal_iterator::{Internal, InternalIterator, IteratorExt};
use rand::Rng;

use crate::board::{Board, BoardAvailableMoves, Outcome, Player, UnitSymmetryBoard};
use crate::util::bot_game::Replay;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    max_repetitions: Option<u16>,
    max_moves_without_pawn_or_capture: Option<u16>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    inner: chess::Board,
    rules: Rules,

    // 0 if this is the first time this position was reached
    //TODO remove pub and create getters
    pub repetitions: u16,
    pub non_pawn_or_capture_moves: u16,

    history: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct ParseMoveError {
    pub mv: String,
    pub error: chess::Error,
}

impl ChessBoard {
    pub fn default_with_rules(rules: Rules) -> Self {
        Self::new_without_history(chess::Board::default(), rules)
    }

    pub fn new_without_history_fen(fen: &str, rules: Rules) -> Self {
        Self::new_without_history(chess::Board::from_str(fen).unwrap(), rules)
    }

    /// Construct a new board from an inner chess board, without any history (including repetition and reset counters).
    /// To get a board _with_ history start from the initial board and play the moves on it instead.
    pub fn new_without_history(inner: chess::Board, rules: Rules) -> Self {
        ChessBoard {
            inner,
            rules,
            repetitions: 0,
            non_pawn_or_capture_moves: 0,
            history: vec![],
        }
    }

    pub fn inner(&self) -> &chess::Board {
        &self.inner
    }

    pub fn parse_move(&self, mv: &str) -> Result<ChessMove, ParseMoveError> {
        // first try parsing it as a pgn move
        if let Ok(mv) = ChessMove::from_str(mv) {
            return Ok(mv);
        }

        // the chess crate move parsing is kind of strange, so we need to help it a bit
        let removed_chars: &[char] = &['=', '+', '#'];
        let mv = if mv.contains(removed_chars) {
            Cow::from(mv.replace(removed_chars, ""))
        } else {
            Cow::from(mv)
        };

        match ChessMove::from_san(self.inner(), &mv) {
            Ok(mv) => Ok(mv),
            Err(original_err) => {
                // try appending e.p. to get it to parse an en passant move
                let mv_ep = mv.to_owned() + " e.p.";
                ChessMove::from_san(self.inner(), &mv_ep).map_err(|_| ParseMoveError {
                    mv: mv.into_owned(),
                    error: original_err,
                })
            }
        }
    }

    pub fn to_san(&self, mv: ChessMove) -> String {
        assert!(self.is_available_move(mv));

        let piece = match self.inner.piece_on(mv.get_source()).unwrap() {
            Piece::Pawn => "".to_string(),
            Piece::King => match (mv.get_source().get_file(), mv.get_dest().get_file()) {
                (File::E, File::G) => return "O-O".to_string(),
                (File::E, File::C) => return "O-O-O".to_string(),
                _ => "K".to_string(),
            },
            piece => piece.to_string(Color::White),
        };

        let mut result = String::new();
        let f = &mut result;

        write!(f, "{}{}{}", piece, mv.get_source(), mv.get_dest()).unwrap();
        if let Some(promotion) = mv.get_promotion() {
            write!(f, "={}", promotion.to_string(Color::White)).unwrap();
        }

        result
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        ChessBoard::default_with_rules(Rules::default())
    }
}

impl Board for ChessBoard {
    type Move = ChessMove;

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

        // keep track of stats for reversible moves
        let old_side_to_move = self.inner.side_to_move();
        let old_castle_rights = self.inner.castle_rights(old_side_to_move);

        let moved_piece = self.inner.piece_on(mv.get_source()).unwrap();
        let was_capture = self.inner.color_on(mv.get_dest()).is_some();
        let was_pawn_move = moved_piece == Piece::Pawn;

        // make the move
        let old_inner = self.inner;
        self.inner = old_inner.make_move_new(mv);

        // collect more stats
        let removed_castle = old_castle_rights != self.inner.castle_rights(old_side_to_move);

        // update move counter
        if was_capture || was_pawn_move {
            self.non_pawn_or_capture_moves = 0
        } else {
            self.non_pawn_or_capture_moves += 1
        };

        // update history
        let reset_history = was_capture || was_pawn_move || removed_castle || self.rules.max_repetitions.is_none();
        if reset_history {
            self.history.clear()
        } else {
            self.history.push(self.inner.get_hash())
        }

        // update repetition counter based on history
        //TODO we only need to check every other board position here
        self.repetitions = self.history.iter().filter(|&&h| self.inner.get_hash() == h).count() as u16;
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.rules.is_draw(self) {
            Some(Outcome::Draw)
        } else {
            match self.inner.status() {
                BoardStatus::Ongoing => None,
                BoardStatus::Stalemate => Some(Outcome::Draw),
                BoardStatus::Checkmate => Some(Outcome::WonBy(self.next_player().other())),
            }
        }
    }

    fn can_lose_after_move() -> bool {
        false
    }
}

impl UnitSymmetryBoard for ChessBoard {}

#[derive(Debug)]
pub struct AllMoveIterator;

impl InternalIterator for AllMoveIterator {
    type Item = ChessMove;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        for from in chess::ALL_SQUARES {
            for to in chess::ALL_SQUARES {
                f(ChessMove::new(from, to, None))?;

                for piece in chess::PROMOTION_PIECES {
                    f(ChessMove::new(from, to, Some(piece)))?;
                }
            }
        }

        ControlFlow::Continue(())
    }
}

impl<'a> BoardAvailableMoves<'a, ChessBoard> for ChessBoard {
    type AllMoveIterator = AllMoveIterator;
    type MoveIterator = Internal<MoveGen>;

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
    pub fn unlimited() -> Self {
        Rules {
            max_repetitions: None,
            max_moves_without_pawn_or_capture: None,
        }
    }

    pub fn is_draw(self, board: &ChessBoard) -> bool {
        let draw_repetitions = self.max_repetitions.map_or(false, |m| board.repetitions >= m);
        let draw_reversible = self
            .max_moves_without_pawn_or_capture
            .map_or(false, |m| board.non_pawn_or_capture_moves >= m);
        let only_kings = board.inner.combined().popcnt() == 2;
        draw_repetitions || draw_reversible || only_kings
    }
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            max_repetitions: Some(3),
            max_moves_without_pawn_or_capture: Some(100),
        }
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChessBoard {{ inner: \"{}\", cr: {}, ci: {}, h: {}, rules: {:?} }}",
            self.inner,
            self.repetitions,
            self.non_pawn_or_capture_moves,
            self.history.len(),
            self.rules
        )
    }
}

pub fn chess_game_to_pgn(white: &str, black: &str, start: &ChessBoard, moves: &[ChessMove]) -> String {
    let mut result = String::new();
    let f = &mut result;

    writeln!(f, "[White \"{}\"]", white).unwrap();
    writeln!(f, "[Black \"{}\"]", black).unwrap();
    writeln!(f, "[FEN \"{}\"]", start.inner).unwrap();

    let mut board = start.clone();

    for (i, &mv) in moves.iter().enumerate() {
        if i % 2 == 0 {
            write!(f, "{}. ", 1 + i / 2).unwrap();
        }

        write!(f, "{} ", board.to_san(mv)).unwrap();

        board.play(mv);
    }

    result
}

impl Replay<ChessBoard> {
    pub fn to_pgn(&self) -> String {
        let full_l = format!("L: {}", self.debug_l);
        let full_r = format!("R: {}", self.debug_r);

        let (white, black) = match self.player_l {
            Player::A => (&full_l, &full_r),
            Player::B => (&full_r, &full_l),
        };
        chess_game_to_pgn(white, black, &self.start, &self.moves)
    }
}
