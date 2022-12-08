use std::borrow::Cow;
use std::fmt::{Debug, Write};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::ControlFlow;
use std::str::FromStr;

use chess::{BoardStatus, ChessMove, Color, File, MoveGen, Piece, Square};
use internal_iterator::{Internal, InternalIterator, IteratorExt};
use rand::Rng;

use crate::board::{AllMovesIterator, Alternating, Board, BoardMoves, Outcome, Player};
use crate::impl_unit_symmetry_board;
use crate::util::bot_game::Replay;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    max_repetitions: Option<u16>,
    max_moves_without_pawn_or_capture: Option<u16>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ChessBoard {
    rules: Rules,

    inner: chess::Board,
    history: Vec<chess::Board>,

    // cached values
    non_pawn_or_capture_moves: u16,
    repetitions: u16,
    outcome: Option<Outcome>,
}

#[derive(Debug, Clone)]
pub struct ParseMoveError {
    pub board: ChessBoard,
    pub mv: String,
    pub error: Option<chess::Error>,
    pub parsed_as_but_not_available: Option<ChessMove>,
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
            rules,
            inner,
            history: vec![],
            non_pawn_or_capture_moves: 0,
            repetitions: 0,
            outcome: None,
        }
    }

    pub fn parse_move(&self, mv_str: &str) -> Result<ChessMove, ParseMoveError> {
        assert!(
            !self.is_done(),
            "Cannot parse move {:?} for done board {:?}",
            mv_str,
            self
        );

        let mv = parse_move_inner_impl(self, mv_str)?;

        // fix alternative castling move representation
        let current = &self.inner;
        let from = mv.get_source();
        let to = mv.get_dest();
        let next = current.side_to_move();
        let is_alternative_castling_format = current.piece_on(from) == Some(Piece::King)
            && current.piece_on(to) == Some(Piece::Rook)
            && current.color_on(from) == Some(next)
            && current.color_on(to) == Some(next);
        let mv = if is_alternative_castling_format {
            assert!(from.get_rank() == to.get_rank() && mv.get_promotion().is_none());
            let from_file = from.get_file().to_index() as i8;
            let direction = (to.get_file().to_index() as i8 - from_file).signum();
            let to_file = File::from_index((from_file + 2 * direction) as usize);
            let actual_to = Square::make_square(to.get_rank(), to_file);
            ChessMove::new(from, actual_to, None)
        } else {
            mv
        };

        // ensure the move is actually available
        if self.is_available_move(mv) {
            Ok(mv)
        } else {
            Err(ParseMoveError {
                board: self.clone(),
                mv: mv_str.to_owned(),
                error: None,
                parsed_as_but_not_available: Some(mv),
            })
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

    /// Count how often the given position occurs in this boards history.
    pub fn repetitions_for(&self, board: &chess::Board) -> usize {
        // TODO we only need to check half of the history, depending on the color xor
        self.history.iter().filter(|&h| h == board).count()
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn inner(&self) -> &chess::Board {
        &self.inner
    }

    pub fn history(&self) -> &Vec<chess::Board> {
        &self.history
    }

    pub fn non_pawn_or_capture_moves(&self) -> u16 {
        self.non_pawn_or_capture_moves
    }

    pub fn repetitions(&self) -> u16 {
        self.repetitions
    }
}

fn parse_move_inner_impl(board: &ChessBoard, mv_str: &str) -> Result<ChessMove, ParseMoveError> {
    // first try parsing it as a pgn move
    if let Ok(mv) = ChessMove::from_str(mv_str) {
        return Ok(mv);
    }

    // the chess crate move parsing is kind of strange, so we need to help it a bit
    let removed_chars: &[char] = &['=', '+', '#'];
    let mv = if mv_str.contains(removed_chars) {
        Cow::from(mv_str.replace(removed_chars, ""))
    } else {
        Cow::from(mv_str)
    };

    match ChessMove::from_san(board.inner(), &mv) {
        Ok(mv) => Ok(mv),
        Err(original_err) => {
            // try appending e.p. to get it to parse an en passant move
            let mv_ep = mv.to_owned() + " e.p.";
            ChessMove::from_san(board.inner(), &mv_ep).map_err(|_| ParseMoveError {
                board: board.clone(),
                mv: mv.into_owned(),
                error: Some(original_err),
                parsed_as_but_not_available: None,
            })
        }
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
        assert!(self.is_available_move(mv), "{:?} is not available on {:?}", mv, self);

        // keep track of stats for reversible moves
        let prev = self.inner;
        let old_side_to_move = prev.side_to_move();
        let old_castle_rights = prev.castle_rights(old_side_to_move);

        let moved_piece = prev.piece_on(mv.get_source()).unwrap();
        let was_capture = prev.color_on(mv.get_dest()).is_some();
        let was_pawn_move = moved_piece == Piece::Pawn;

        // make the move
        self.inner = prev.make_move_new(mv);

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
                BoardStatus::Ongoing => None,
                BoardStatus::Stalemate => Some(Outcome::Draw),
                BoardStatus::Checkmate => Some(Outcome::WonBy(self.next_player().other())),
            }
        }
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
    type AvailableMovesIterator = Internal<MoveGen>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
        assert!(!self.is_done());
        MoveGen::new_legal(self.inner()).into_internal()
    }
}

impl InternalIterator for AllMovesIterator<ChessBoard> {
    type Item = ChessMove;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        //TODO we're yielding a *lot* of impossible moves here
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
