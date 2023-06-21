use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use std::sync::Arc;

use internal_iterator::InternalIterator;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::games::scrabble::basic::Deck;
use crate::games::scrabble::grid::ScrabbleGrid;
use crate::games::scrabble::movegen::{Move, Set};
use crate::impl_unit_symmetry_board;

#[derive(Clone)]
pub struct ScrabbleBoard {
    pub grid: ScrabbleGrid,

    pub next_player: Player,
    // TODO come up with some general per-player storage wrapper
    pub deck_a: Deck,
    pub deck_b: Deck,
    pub score_a: u32,
    pub score_b: u32,
    pub exchange_count: u8,

    pub set: Arc<Set>,
}

impl Default for ScrabbleBoard {
    fn default() -> Self {
        ScrabbleBoard {
            grid: ScrabbleGrid::default(),
            next_player: Player::A,
            deck_a: Default::default(),
            deck_b: Default::default(),
            score_a: 0,
            score_b: 0,
            exchange_count: 0,
            set: Arc::new(Default::default()),
        }
    }
}

impl ScrabbleBoard {
    pub fn new(
        grid: ScrabbleGrid,
        next_player: Player,
        deck_a: Deck,
        deck_b: Deck,
        score_a: u32,
        score_b: u32,
        exchange_count: u8,
        set: Arc<Set>,
    ) -> Self {
        assert!(exchange_count <= 4);
        Self {
            grid,
            next_player,
            deck_a,
            deck_b,
            score_a,
            score_b,
            exchange_count,
            set,
        }
    }

    fn eq_key(&self) -> impl Eq + '_ {
        (
            self.deck_a,
            self.deck_b,
            self.score_a,
            self.score_b,
            self.next_player,
            &self.grid,
            self.set.as_fst().as_bytes(),
        )
    }

    pub fn next_deck(&self) -> Deck {
        match self.next_player {
            Player::A => self.deck_a,
            Player::B => self.deck_b,
        }
    }

    fn next_deck_score_mut(&mut self) -> (&mut Deck, &mut u32) {
        match self.next_player {
            Player::A => (&mut self.deck_a, &mut self.score_a),
            Player::B => (&mut self.deck_b, &mut self.score_b),
        }
    }
}

impl Board for ScrabbleBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;
        let deck = self.next_deck();
        Ok(self.grid.can_play(&self.set, mv, deck).is_ok())
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        // TODO implement letter swapping turn

        self.check_done()?;

        let deck = self.next_deck();
        match self.grid.play(&self.set, mv, deck) {
            Ok(new_deck) => {
                let (deck, score) = self.next_deck_score_mut();
                *deck = new_deck;
                *score += mv.score;

                self.next_player = self.next_player.other();
                Ok(())
            }
            Err(_) => Err(PlayError::UnavailableMove),
        }
    }

    fn outcome(&self) -> Option<Outcome> {
        let end = self.exchange_count >= 4 || self.deck_a.is_empty() || self.deck_b.is_empty();
        if end {
            Some(Outcome::from_score(self.score_a, self.score_b))
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
    fn try_for_each<R, F>(self, f: F) -> ControlFlow<R>
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

    fn try_for_each<R, F>(self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let board = self.board();
        let set = &board.set;
        let deck = board.next_deck();

        board.grid.available_moves(set, deck).try_for_each(f)
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
            "ScrabbleBoard {{ next_player: {:?}, deck_a: {:?}, deck_b: {:?}, score_a: {:?}, score_b: {:?}, exchange_count: {} }}",
            self.next_player, self.deck_a, self.deck_b, self.score_a, self.score_b, self.exchange_count
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
