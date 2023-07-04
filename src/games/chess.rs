use std::fmt::{Debug, Write};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::ControlFlow;
use std::str::FromStr;

use cozy_chess::{BitBoard, Color, FenParseError, File, GameStatus, Move, Piece};
use internal_iterator::InternalIterator;

use crate::board::{
    AllMovesIterator, Alternating, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::impl_unit_symmetry_board;
use crate::util::bot_game::Replay;

type InnerBoard = cozy_chess::Board;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    max_repetitions: Option<u16>,
    max_moves_without_pawn_or_capture: Option<u16>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    rules: Rules,

    // we don't use the half-move counter in the inner board, since it's hardcoded to stop at 100
    //   while we have to support arbitrary rules
    inner: InnerBoard,
    history: Vec<InnerBoard>,

    // cached values
    non_pawn_or_capture_moves: u16,
    repetitions: u16,
    outcome: Option<Outcome>,
}

#[derive(Debug)]
pub struct ParseMoveError {
    pub board: ChessBoard,
    pub mv: String,
    pub kind: ParseMoveErrorKind,
}

#[derive(Debug)]
pub enum ParseMoveErrorKind {
    BoardDone,
    Unavailable(Move),
    ParseError,
}

impl ChessBoard {
    pub fn default_with_rules(rules: Rules) -> Self {
        Self::new_without_history(InnerBoard::default(), rules)
    }

    pub fn new_without_history_fen(fen: &str, rules: Rules) -> Result<Self, FenParseError> {
        Ok(Self::new_without_history(InnerBoard::from_fen(fen, false)?, rules))
    }

    /// Construct a new board from an inner chess board, without any history (including repetition and reset counters).
    /// To get a board _with_ history start from the initial board and play the moves on it instead.
    pub fn new_without_history(inner: InnerBoard, rules: Rules) -> Self {
        // TODO get this working correctly
        assert_eq!(
            rules.max_moves_without_pawn_or_capture,
            Some(100),
            "for now only exact 50-move rule is supported"
        );
        ChessBoard {
            rules,
            inner,
            history: vec![],
            non_pawn_or_capture_moves: 0,
            repetitions: 0,
            outcome: None,
        }
    }

    pub fn parse_move(&self, mv_str: &str) -> Result<Move, Box<ParseMoveError>> {
        let err = |kind| ParseMoveError {
            board: self.clone(),
            mv: mv_str.to_owned(),
            kind,
        };
        self.check_done().map_err(|_| err(ParseMoveErrorKind::BoardDone))?;

        // this already ignored any extra characters for us (which is a bit sketchy)
        let mv = Move::from_str(mv_str).map_err(|_| err(ParseMoveErrorKind::ParseError))?;

        // ensure the move is actually available
        //   we can unwrap here since we already checked that the board is not done
        if self.is_available_move(mv).unwrap() {
            Ok(mv)
        } else {
            Err(Box::new(err(ParseMoveErrorKind::Unavailable(mv))))
        }
    }

    pub fn to_san(&self, mv: Move) -> Result<String, PlayError> {
        self.check_can_play(mv)?;

        let piece = match self.inner.piece_on(mv.from).unwrap() {
            Piece::Pawn => "".to_string(),
            Piece::King => match (mv.from.file(), mv.to.file()) {
                (File::E, File::G) => return Ok("O-O".to_string()),
                (File::E, File::C) => return Ok("O-O-O".to_string()),
                _ => "K".to_string(),
            },
            piece => {
                let c: char = piece.into();
                c.to_ascii_uppercase().to_string()
            }
        };

        let mut result = String::new();
        let f = &mut result;

        write!(f, "{}{}{}", piece, mv.from, mv.to).unwrap();
        if let Some(promotion) = mv.promotion {
            let c: char = promotion.into();
            write!(f, "={}", c.to_ascii_uppercase()).unwrap();
        }

        Ok(result)
    }

    /// Count how often the given position occurs in this boards history.
    pub fn repetitions_for(&self, board: &InnerBoard) -> usize {
        // TODO we only need to check half of the history, depending on the color xor
        self.history.iter().filter(|&h| h.same_position(board)).count()
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn inner(&self) -> &InnerBoard {
        &self.inner
    }

    pub fn history(&self) -> &Vec<InnerBoard> {
        &self.history
    }

    pub fn non_pawn_or_capture_moves(&self) -> u16 {
        self.non_pawn_or_capture_moves
    }

    pub fn repetitions(&self) -> u16 {
        self.repetitions
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        ChessBoard::default_with_rules(Rules::default())
    }
}

impl Board for ChessBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        color_to_player(self.inner.side_to_move())
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;
        Ok(self.inner.is_legal(mv))
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_done()?;

        // keep track of stats for reversible moves
        let prev = self.inner.clone();
        let old_side_to_move = prev.side_to_move();
        let old_castle_rights = prev.castle_rights(old_side_to_move);

        let moved_piece = prev.piece_on(mv.from).unwrap();
        let was_capture = prev.color_on(mv.to).is_some();
        let was_pawn_move = moved_piece == Piece::Pawn;

        // make the move
        self.inner.try_play(mv).map_err(|_| PlayError::UnavailableMove)?;

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
            self.history.push(prev)
        }

        // update repetition counter based on history
        self.repetitions = self.repetitions_for(&self.inner) as u16;

        //update outcome
        self.outcome = if self.rules.is_draw(self) {
            Some(Outcome::Draw)
        } else {
            match self.inner.status() {
                GameStatus::Won => Some(Outcome::WonBy(self.next_player().other())),
                GameStatus::Drawn => Some(Outcome::Draw),
                GameStatus::Ongoing => None,
            }
        };

        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn can_lose_after_move() -> bool {
        false
    }
}

impl Alternating for ChessBoard {}

impl_unit_symmetry_board!(ChessBoard);

impl<'a> BoardMoves<'a, ChessBoard> for ChessBoard {
    type AllMovesIterator = AllMovesIterator<ChessBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, ChessBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        AvailableMovesIterator::new(self)
    }
}

impl InternalIterator for AllMovesIterator<ChessBoard> {
    type Item = Move;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        //TODO we're yielding a *lot* of impossible moves here
        for from in BitBoard::FULL {
            for to in BitBoard::FULL {
                f(Move {
                    from,
                    to,
                    promotion: None,
                })?;

                for piece in Piece::ALL {
                    f(Move {
                        from,
                        to,
                        promotion: Some(piece),
                    })?;
                }
            }
        }

        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, ChessBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let mut break_value = None;
        let did_break = self
            .board()
            .inner
            .generate_moves(|piece| match piece.into_iter().try_for_each(&mut f) {
                ControlFlow::Continue(()) => false,
                ControlFlow::Break(value) => {
                    debug_assert!(break_value.is_none());
                    break_value = Some(value);
                    true
                }
            });

        debug_assert_eq!(break_value.is_some(), did_break);
        match break_value {
            None => ControlFlow::Continue(()),
            Some(value) => ControlFlow::Break(value),
        }
    }

    fn count(self) -> usize {
        let mut result = 0;
        self.board().inner.generate_moves(|piece| {
            result += piece.len();
            false
        });
        result
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
        let only_kings = board.inner.colors(Color::Black).len() == 1 && board.inner.colors(Color::White).len() == 1;
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

impl Debug for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ChessBoard {{ inner: \"{}\", rep: {}, non: {}, hist: {}, rules: {:?} }}",
            self.inner,
            self.repetitions,
            self.non_pawn_or_capture_moves,
            self.history.len(),
            self.rules
        )
    }
}

pub fn chess_game_to_pgn(white: &str, black: &str, start: &ChessBoard, moves: &[Move]) -> String {
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

        write!(f, "{} ", board.to_san(mv).unwrap()).unwrap();

        board.play(mv).unwrap();
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
