use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::games::go::chains::Chains;
use crate::games::go::tile::Tile;
use crate::games::go::Rules;
use crate::impl_unit_symmetry_board;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct GoBoard {
    rules: Rules,
    chains: Option<Chains>,
    next_player: Player,
    state: State,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Move {
    Pass,
    Place(Tile),
}

#[derive(Debug, Copy, Clone)]
pub struct Score {
    pub a: u32,
    pub b: u32,
}

impl Score {
    // TODO komi?
    pub fn to_outcome(self) -> Outcome {
        match self.a.cmp(&self.b) {
            Ordering::Less => Outcome::WonBy(Player::A),
            Ordering::Equal => Outcome::Draw,
            Ordering::Greater => Outcome::WonBy(Player::B),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum State {
    Normal,
    Passed,
    Done(Outcome),
}

impl GoBoard {
    pub fn new(size: u8, rules: Rules) -> GoBoard {
        GoBoard {
            rules,
            chains: Some(Chains::new(size)),
            next_player: Player::A,
            state: State::Normal,
        }
    }

    pub(super) fn from_parts(rules: Rules, chains: Chains, next_player: Player, state: State) -> GoBoard {
        GoBoard {
            rules,
            chains: Some(chains),
            next_player,
            state,
        }
    }

    pub fn size(&self) -> u8 {
        self.chains().size()
    }

    pub fn area(&self) -> u16 {
        self.chains().area()
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn chains(&self) -> &Chains {
        self.chains
            .as_ref()
            .expect("Board is in invalid state after failed play")
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.chains().stone_at(tile)
    }

    pub fn current_score(&self) -> Score {
        self.chains().score()
    }
}

impl Board for GoBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        match mv {
            Move::Pass => Ok(true),
            // TODO ensure the board would not repeat by playing at `tile`
            Move::Place(tile) => {
                if !tile.exists(self.size()) {
                    Ok(false)
                } else {
                    Ok(self.stone_at(tile).is_none())
                }
            }
        }
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_can_play(mv)?;
        let curr = self.next_player;
        let other = curr.other();

        match mv {
            Move::Pass => {
                self.next_player = other;
                self.state = match self.state {
                    State::Normal => State::Passed,
                    State::Passed => State::Done(self.current_score().to_outcome()),
                    State::Done(_) => unreachable!(),
                };
            }
            Move::Place(tile) => {
                let chains = self.chains.take().expect("Board is in invalid state after failed play");
                // TODO handle this error properly, eg. "unavailable move"
                let new_chains = chains.place_tile_full(tile, curr, &self.rules).unwrap().chains;
                self.chains = Some(new_chains);

                self.next_player = other;
                self.state = State::Normal;
            }
        }

        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        match self.state {
            State::Normal | State::Passed => None,
            State::Done(outcome) => Some(outcome),
        }
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl<'a> BoardMoves<'a, GoBoard> for GoBoard {
    type AllMovesIterator = AllMovesIterator<GoBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, GoBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        AvailableMovesIterator::new(self)
    }
}

impl InternalIterator for AllMovesIterator<GoBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        f(Move::Pass)?;
        for tile in Tile::all(Chains::MAX_SIZE) {
            f(Move::Place(tile))?;
        }
        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, GoBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let board = self.board();

        // TODO remove repeating moves

        // we already know the board is not done at this point
        //  so we can just yield all empty tiles (and the pass move)
        f(Move::Pass)?;
        for tile in Tile::all(board.size()) {
            if board.stone_at(tile).is_none() {
                f(Move::Place(tile))?;
            }
        }
        ControlFlow::Continue(())
    }

    fn count(self) -> usize {
        let board = self.board();

        // TODO remove repeating moves
        // TODO write faster function for this
        1 + Tile::all(board.size())
            .filter(|&tile| board.stone_at(tile).is_none())
            .count()
    }
}

// TODO implement proper symmetry
impl_unit_symmetry_board!(GoBoard);
