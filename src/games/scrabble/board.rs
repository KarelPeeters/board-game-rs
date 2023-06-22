use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use std::sync::Arc;

use internal_iterator::InternalIterator;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::games::scrabble::basic::Deck;
use crate::games::scrabble::grid::ScrabbleGrid;
use crate::games::scrabble::movegen::{PlaceMove, Set};
use crate::games::scrabble::zobrist::Zobrist;
use crate::impl_unit_symmetry_board;
use crate::pov::{NonPov, PlayerBox};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Move {
    Place(PlaceMove),
    // TODO include which letters to exchange
    Exchange,
}

#[derive(Clone)]
pub struct ScrabbleBoard {
    grid: ScrabbleGrid,

    next_player: Player,
    decks: PlayerBox<Deck>,
    scores: PlayerBox<u32>,
    exchange_count: u8,

    set: Arc<Set>,
}

impl ScrabbleBoard {
    pub fn new(
        grid: ScrabbleGrid,
        next_player: Player,
        decks: PlayerBox<Deck>,
        scores: PlayerBox<u32>,
        exchange_count: u8,
        set: Arc<Set>,
    ) -> Self {
        assert!(exchange_count <= 4);
        Self {
            grid,
            next_player,
            decks,
            scores,
            exchange_count,
            set,
        }
    }

    fn eq_key(&self) -> impl Eq + '_ {
        (
            self.next_player,
            self.decks,
            self.scores,
            &self.grid,
            self.set.as_fst().as_bytes(),
        )
    }

    pub fn decks(&self) -> PlayerBox<Deck> {
        self.decks
    }

    pub fn set_decks(&mut self, decks: PlayerBox<Deck>) {
        self.decks = decks;
    }

    pub fn scores(&self) -> PlayerBox<u32> {
        self.scores
    }

    pub fn set_scores(&mut self, scores: PlayerBox<u32>) {
        self.scores = scores;
    }

    pub fn grid(&self) -> &ScrabbleGrid {
        &self.grid
    }

    pub fn set(&self) -> &Arc<Set> {
        &self.set
    }

    pub fn exchange_count(&self) -> u8 {
        self.exchange_count
    }

    pub fn zobrist_pov_without_score(&self) -> Zobrist {
        let mut result = self.grid.zobrist();

        let decks = self.decks.pov(self.next_player);
        result ^= Zobrist::for_deck(true, decks.pov);
        result ^= Zobrist::for_deck(false, decks.other);

        result ^= Zobrist::for_exchange_count(self.exchange_count);

        result
    }
}

impl Board for ScrabbleBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        let is_available = match mv {
            Move::Place(mv) => {
                let deck = self.decks[self.next_player];
                self.grid.simulate_play(mv, deck).is_ok()
            }
            Move::Exchange => true,
        };
        Ok(is_available)
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_done()?;

        match mv {
            Move::Place(mv) => {
                let player = self.next_player;
                let deck = self.decks[player];

                match self.grid.play(&self.set, mv, deck) {
                    Ok(new_deck) => {
                        self.scores[player] += mv.score;
                        self.decks[player] = new_deck;
                        self.next_player = player.other();
                        Ok(())
                    }
                    Err(_) => Err(PlayError::UnavailableMove),
                }
            }
            Move::Exchange => {
                self.exchange_count += 1;
                assert!(self.exchange_count <= 4);
                self.next_player = self.next_player.other();
                Ok(())
            }
        }
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.exchange_count >= 4 || self.decks.a.is_empty() || self.decks.b.is_empty() {
            Some(Outcome::from_scores(self.scores))
        } else {
            None
        }
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

// TODO proper symmetry
impl_unit_symmetry_board!(ScrabbleBoard);

impl<'a> BoardMoves<'a, ScrabbleBoard> for ScrabbleBoard {
    type AllMovesIterator = AllMovesIterator<ScrabbleBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, ScrabbleBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        AvailableMovesIterator::new(self)
    }
}

impl InternalIterator for AllMovesIterator<ScrabbleBoard> {
    type Item = Move;
    fn try_for_each<R, F>(self, _: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        // TODO we can't really implement this, roughly 15x15x26^7 items
        // TODO make this optional in board?
        todo!()
    }
}

impl InternalIterator for AvailableMovesIterator<'_, ScrabbleBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let board = self.board();
        let set = &board.set;
        let deck = board.decks[board.next_player];

        // place moves
        board
            .grid
            .available_moves(set, deck)
            .try_for_each(|mv| f(Move::Place(mv)))?;

        // TODO put these first?
        // exchange moves
        f(Move::Exchange)?;

        ControlFlow::Continue(())
    }
}

impl Eq for ScrabbleBoard {}

impl PartialEq for ScrabbleBoard {
    fn eq(&self, other: &Self) -> bool {
        self.eq_key() == other.eq_key()
    }
}

impl Debug for ScrabbleBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ScrabbleBoard {{ next_player: {:?}, decks: {:?}, scores: {:?}, exchange_count: {}, outcome: {:?} }}",
            self.next_player,
            self.decks,
            self.scores,
            self.exchange_count,
            self.outcome(),
        )
    }
}

impl Display for ScrabbleBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self)?;
        writeln!(f, "{}", self.grid)?;
        Ok(())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
