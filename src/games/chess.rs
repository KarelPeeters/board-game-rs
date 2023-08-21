use std::fmt::{Debug, Write};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::ControlFlow;
use std::str::FromStr;

use cozy_chess::{BitBoard, Color, FenParseError, File, GameStatus, Move, Piece, PieceMoves, Rank, Square};
use internal_iterator::InternalIterator;
use lazy_static::lazy_static;
use regex_lite::Regex;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ChessMove {
    Normal { piece: Piece, from: Square, to: Square },
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

impl ChessBoard {
    pub fn default_with_rules(rules: Rules) -> Self {
        Self::from_inner(InnerBoard::default(), rules)
    }

    pub fn from_fen(fen: &str, rules: Rules) -> Result<Self, FenParseError> {
        Ok(Self::from_inner(InnerBoard::from_fen(fen, false)?, rules))
    }

    /// Construct a new board from an inner chess board, without any history (including repetition and reset counters).
    /// To get a board _with_ history start from the initial board and play the moves on it instead.
    pub fn from_inner(inner: InnerBoard, rules: Rules) -> Self {
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

    pub fn parse_move(&self, _: &str) -> Result<Move, Box<ParseSanMoveError>> {
        // // TODO convert case, try UCI and SAN
        todo!()
    }

    pub fn to_san(&self, mv: Move) -> Result<String, PlayError> {
        todo!();

        self.check_can_play(mv)?;

        let piece = match self.inner.piece_on(mv.from).unwrap() {
            Piece::Pawn => String::new(),
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

pub fn move_to_uci_str(mv: Move) -> String {
    let Move { from, to, promotion } = mv;

    match promotion {
        None => format!("{}{}", from, to),
        Some(promotion) => format!("{}{}{}", from, to, promotion),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParseUciMoveError {
    pub mv: String,
}

pub fn move_from_uci_str(s: &str) -> Result<Move, ParseUciMoveError> {
    // TODO castling is broken :(
    if s.len() == 4 || s.len() == 5 {
        if let Ok(mv) = Move::from_str(s) {
            if s.len() == 4 || mv.promotion.is_some() {
                return Ok(mv);
            }
        }
    }

    Err(ParseUciMoveError { mv: s.to_owned() })
}

// TODO check, checkmate?
pub fn move_to_san_str(board: &ChessBoard, mv: Move) -> Result<String, PlayError> {
    board.check_can_play(mv)?;

    let Move { from, to, promotion } = mv;
    let inner = board.inner();

    let piece = inner.piece_on(from).unwrap();
    let captured = inner.piece_on(to);

    // castling, encoded as king capturing own rook
    if piece == Piece::King && captured == Some(Piece::Rook) && inner.color_on(to) == Some(inner.side_to_move()) {
        return match to.file() {
            File::G => Ok("O-O-O".to_owned()),
            File::C => Ok("O-O".to_owned()),
            _ => unreachable!(),
        };
    }

    let result = if piece != Piece::Pawn {
        // move piece

        let mut matching_file = 0;
        let mut matching_rank = 0;

        let mask = inner.colored_pieces(inner.side_to_move(), piece);

        inner.generate_moves_for(mask, |list| {
            matching_file += (list.to & to.file().bitboard()).len();
            matching_rank += (list.to & to.rank().bitboard()).len();

            // stop if we already need both
            matching_file > 1 && matching_rank > 1
        });

        debug_assert!(matching_file > 0 && matching_rank > 0);
        let piece_str = piece.to_string().to_ascii_uppercase();
        let capture_str = if captured.is_some() { "x" } else { "" };

        if matching_file == 1 && matching_rank == 1 {
            format!("{}{}{}", piece_str, capture_str, to)
        } else if matching_file == 1 {
            format!("{}{}{}{}", piece_str, from.file(), capture_str, to)
        } else if matching_rank == 1 {
            format!("{}{}{}{}", piece_str, from.rank(), capture_str, to)
        } else {
            format!("{}{}{}{}", piece_str, from, capture_str, to)
        }
    } else {
        let promotion_str = promotion.map_or("".to_string(), |p| p.to_string().to_ascii_uppercase());

        if captured.is_some() || from.file() != to.file() {
            // pawn capture (including en passant)
            let mut matching_rank = 0;
            let mask = from.file().bitboard() & inner.colored_pieces(inner.side_to_move(), Piece::Pawn);
            inner.generate_moves_for(mask, |list| {
                matching_rank += list.len();
                matching_rank > 1
            });

            debug_assert!(matching_rank > 0);
            if matching_rank == 1 {
                format!("{}x{}{}", from.file(), to, promotion_str)
            } else {
                format!("{}x{}{}", from, to, promotion_str)
            }
        } else {
            // pawn push
            format!("{}{}", to, promotion_str)
        }
    };
    Ok(result)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParseSanMoveError {
    Parse,

    NoMatchingMove,
    MultipleMatchingMoves,
    CaptureMismatch,

    UnavailableMove(Move),
    BoardDone,
}

// TODO check, checkmate?
pub fn move_from_san_str(board: &ChessBoard, s: &str) -> Result<Move, ParseSanMoveError> {
    if board.is_done() {
        return Err(ParseSanMoveError::BoardDone);
    }

    let mv = if s == "O-O" || s == "O-O-O" {
        let back_rank = Rank::First.relative_to(board.inner().side_to_move());
        let rook_file = if s == "O-O" { File::G } else { File::C };
        Move {
            from: Square::new(File::E, back_rank),
            to: Square::new(rook_file, back_rank),
            promotion: None,
        }
    } else if let Some(captures) = REGEX_MOVE_PIECE.captures(s) {
        println!("Move piece");

        let piece = san_piece_from_str(&captures[1]).unwrap();
        let from_file = captures.get(2).map(|file| File::from_str(file.as_str()).unwrap());
        let from_rank = captures.get(3).map(|rank| Rank::from_str(rank.as_str()).unwrap());
        let capture = captures.get(4).is_some();
        let to = Square::from_str(&captures[5]).unwrap();

        let from = find_san_square_from(&board, piece, from_file, from_rank, to)?;

        let actual_capture = board.inner().piece_on(to).is_some();
        if capture != actual_capture {
            return Err(ParseSanMoveError::CaptureMismatch);
        }

        Move {
            from,
            to,
            promotion: None,
        }
    } else if let Some(captures) = REGEX_MOVE_PAWN_CAPTURE.captures(s) {
        println!("Pawn capture");

        let from_file = File::from_str(&captures[1]).unwrap();
        let from_rank = captures.get(2).map(|rank| Rank::from_str(rank.as_str()).unwrap());
        let to = Square::from_str(&captures[3]).unwrap();
        let promotion = captures.get(4).map(|piece| san_piece_from_str(piece.as_str()).unwrap());

        let from = find_san_square_from(board, Piece::Pawn, Some(from_file), from_rank, to)?;

        Move { from, to, promotion }
    } else if let Some(captures) = REGEX_MOVE_PAWN_PUSH.captures(s) {
        println!("Pawn push");

        let to = Square::from_str(&captures[1]).unwrap();
        let promotion = captures.get(2).map(|piece| san_piece_from_str(piece.as_str()).unwrap());

        let from = find_san_square_from(&board, Piece::Pawn, None, None, to)?;

        Move { from, to, promotion }
    } else {
        return Err(ParseSanMoveError::Parse);
    };

    match board.is_available_move(mv).unwrap() {
        true => Ok(mv),
        false => Err(ParseSanMoveError::UnavailableMove(mv)),
    }
}

// Based on <https://www.chessprogramming.org/Algebraic_Chess_Notation#Standard_Algebraic_Notation_.28SAN.29>.
lazy_static! {
    static ref REGEX_MOVE_PIECE: Regex = Regex::new("^([KQRBN])([a-h])?([1-8])?(x)?([a-h][1-8])$").unwrap();
    static ref REGEX_MOVE_PAWN_CAPTURE: Regex = Regex::new("^([a-h])([1-8])?x([a-h][1-8])([QRBN])?$").unwrap();
    static ref REGEX_MOVE_PAWN_PUSH: Regex = Regex::new("^([a-h][1-8])([QRBN])?$").unwrap();
}

fn find_san_square_from(
    board: &ChessBoard,
    piece: Piece,
    from_file: Option<File>,
    from_rank: Option<Rank>,
    to: Square,
) -> Result<Square, ParseSanMoveError> {
    if let (Some(from_file), Some(from_rank)) = (from_file, from_rank) {
        return Ok(Square::new(from_file, from_rank));
    }

    let inner = board.inner();

    // build a mask of potential source squares, and only consider those moves
    let mut mask = inner.colored_pieces(inner.side_to_move(), piece);
    if let Some(from_file) = from_file {
        mask &= from_file.bitboard();
    }
    if let Some(from_rank) = from_rank {
        mask &= from_rank.bitboard();
    }

    let mut result = None;
    let multiple = inner.generate_moves_for(mask, |list| {
        let PieceMoves {
            piece: list_piece,
            from: list_from,
            to: list_to,
        } = list;

        debug_assert_eq!(piece, list_piece);
        debug_assert!(from_file.map_or(true, |from_file| from_file == list_from.file()));
        debug_assert!(from_rank.map_or(true, |from_rank| from_rank == list_from.rank()));

        if list_to.has(to) {
            if result.is_some() {
                true
            } else {
                result = Some(list_from);
                false
            }
        } else {
            false
        }
    });

    if multiple {
        Err(ParseSanMoveError::MultipleMatchingMoves)
    } else {
        result.ok_or(ParseSanMoveError::NoMatchingMove)
    }
}

fn san_piece_from_str(s: &str) -> Option<Piece> {
    match s {
        "K" => Some(Piece::King),
        "Q" => Some(Piece::Queen),
        "R" => Some(Piece::Rook),
        "B" => Some(Piece::Bishop),
        "N" => Some(Piece::Knight),
        _ => None,
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
            self.non_pawn_or_capture_moves = 0;
        } else {
            self.non_pawn_or_capture_moves += 1;
        };

        // update history
        let reset_history = was_capture || was_pawn_move || removed_castle || self.rules.max_repetitions.is_none();
        if reset_history {
            self.history.clear();
        } else {
            self.history.push(prev);
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
