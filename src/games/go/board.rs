use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use itertools::Itertools;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::games::go::chains::Chains;
use crate::games::go::tile::Tile;
use crate::games::go::{PlacementKind, Rules, SimulatedPlacement, TileOccupied, Zobrist};
use crate::impl_unit_symmetry_board;

#[derive(Clone, Eq, PartialEq)]
pub struct GoBoard {
    rules: Rules,
    chains: Chains,
    next_player: Player,
    state: State,

    // TODO use a hashset instead? or some even better structure?
    //   maybe this can be (partially) shared between board clones?
    history: Vec<Zobrist>,
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
            chains: Chains::new(size),
            next_player: Player::A,
            state: State::Normal,
            history: Vec::new(),
        }
    }

    pub(super) fn from_parts(
        rules: Rules,
        chains: Chains,
        next_player: Player,
        state: State,
        history: Vec<Zobrist>,
    ) -> GoBoard {
        GoBoard {
            rules,
            chains,
            next_player,
            state,
            history,
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
        &self.chains
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn history(&self) -> &[Zobrist] {
        &self.history
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.chains().stone_at(tile)
    }

    pub fn current_score(&self) -> Score {
        self.chains().score()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn without_history(&self) -> Self {
        Self::from_parts(self.rules, self.chains.clone(), self.next_player, self.state, vec![])
    }

    fn is_available_move_sim(&self, sim: SimulatedPlacement) -> bool {
        // check placement kind
        match sim.kind {
            PlacementKind::Normal => {}
            PlacementKind::Capture => {}
            PlacementKind::SuicideSingle => return false,
            PlacementKind::SuicideMulti => {
                if !self.rules.allow_multi_stone_suicide {
                    return false;
                }
            }
        }

        // check history
        //   scan in reverse to hopefully find quicker matches
        if !self.rules.allow_repeating_tiles() && self.history.iter().rev().contains(&sim.zobrist_next) {
            return false;
        }

        true
    }

    /// Full zobrist, including:
    /// * the tiles
    /// * the next player
    /// * the pass state
    pub fn zobrist_full(&self) -> Zobrist {
        // TODO include rules?
        let mut result = self.chains().zobrist();
        result ^= Zobrist::for_player_turn(self.next_player);
        result ^= Zobrist::for_pass_state(self.state);
        result
    }
}

impl Board for GoBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        let result = match mv {
            Move::Pass => true,
            Move::Place(tile) => {
                if !tile.exists(self.size()) {
                    false
                } else {
                    match self.chains.simulate_place_stone(tile, self.next_player) {
                        Ok(sim) => self.is_available_move_sim(sim),
                        Err(TileOccupied) => false,
                    }
                }
            }
        };

        Ok(result)
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        // TODO this is wasteful, we're preparing the chains move twice!
        //   can we save some more state and reuse it?
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
                // update history
                if self.rules.needs_history() {
                    self.history.push(self.chains.zobrist());
                }

                // actually place the tile and check for errors
                let kind = self
                    .chains
                    .place_stone(tile, curr)
                    .expect("Move was not available: tile already occupied");

                // ensure the move was actually valid
                match kind {
                    PlacementKind::Normal => {}
                    PlacementKind::Capture => {}
                    PlacementKind::SuicideSingle => {
                        panic!("Move was not available: single-stone suicide is never allowed")
                    }
                    PlacementKind::SuicideMulti => {
                        if !self.rules.allow_multi_stone_suicide {
                            panic!("Move was not available: multi-stone suicide is not allowed by the current rules")
                        }
                    }
                }
                // TODO check backwards, repetitions are typically close in time
                if !self.rules.allow_repeating_tiles() && self.history.contains(&self.chains.zobrist()) {
                    panic!("Move was not available: repeating tiles is not allowed by the current rules")
                }

                // update auxiliary state
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
        // TODO can this be optimized in any way?
        // this impl is still better than using `BruteforceMoveIterator`,
        //   we only check tiles that are actually in the board

        let board = self.board();

        f(Move::Pass)?;
        for tile in Tile::all(board.size()) {
            if board.is_available_move(Move::Place(tile)).unwrap() {
                f(Move::Place(tile))?;
            }
        }

        ControlFlow::Continue(())
    }

    // TODO add optimized count implementation?
}

// TODO implement proper symmetry
impl_unit_symmetry_board!(GoBoard);

impl Hash for GoBoard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zobrist_full().hash(state);
    }
}
