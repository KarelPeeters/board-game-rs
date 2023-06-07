use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use nohash_hasher::IntSet;
use rand::Rng;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::games::go::chains::Chains;
use crate::games::go::tile::Tile;
use crate::games::go::{PlacementKind, Rules, TileOccupied, Zobrist, GO_MAX_SIZE};
use crate::impl_unit_symmetry_board;
use crate::util::iter::IterExt;

// TODO add must_pass function? maybe even cache the result of that function in board
#[derive(Clone, Eq, PartialEq)]
pub struct GoBoard {
    rules: Rules,
    chains: Chains,
    next_player: Player,
    state: State,
    history: IntSet<Zobrist>,
    komi_2: i16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Move {
    Pass,
    Place(Tile),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Score {
    pub a: u32,
    pub b: u32,
}

impl Score {
    /// Komi is the score bonus given to the second player, white for Go but represented as [Player::B] here.
    pub fn to_outcome(self, komi_2: i16) -> Outcome {
        let score_a = self.a as i32 * 2;
        let total_b = self.b as i32 * 2 + komi_2 as i32;
        match score_a.cmp(&total_b) {
            Ordering::Less => Outcome::WonBy(Player::B),
            Ordering::Equal => Outcome::Draw,
            Ordering::Greater => Outcome::WonBy(Player::A),
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
    /// `komi_2 = int(2 * komi)`, all in favor of white/the second player/[Player::B].
    pub fn new(size: u8, komi_2: i16, rules: Rules) -> GoBoard {
        GoBoard {
            rules,
            chains: Chains::new(size),
            next_player: Player::A,
            state: State::Normal,
            history: Default::default(),
            komi_2,
        }
    }

    pub(super) fn from_parts(
        rules: Rules,
        chains: Chains,
        next_player: Player,
        state: State,
        history: IntSet<Zobrist>,
        komi_2: i16,
    ) -> GoBoard {
        GoBoard {
            rules,
            chains,
            next_player,
            state,
            history,
            komi_2,
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

    pub fn komi_2(&self) -> i16 {
        self.komi_2
    }

    pub fn komi(&self) -> f32 {
        self.komi_2 as f32 / 2.0
    }

    pub fn chains(&self) -> &Chains {
        &self.chains
    }

    pub fn state(&self) -> State {
        self.state
    }

    // TODO add setter for history, and a variant of clone_and_play that takes out the history from the previous board
    //   can be used to optimize things like perft
    pub fn history(&self) -> &IntSet<Zobrist> {
        &self.history
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.chains().stone_at(tile.to_flat(self.size()))
    }

    pub fn empty_tiles(&self) -> impl ExactSizeIterator<Item = Tile> + '_ {
        self.chains()
            .empty_tiles()
            .pure_map(move |tile| tile.to_tile(self.size()))
    }

    pub fn current_score(&self) -> Score {
        self.chains().score()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn without_history(&self) -> Self {
        Self::from_parts(
            self.rules,
            self.chains.clone(),
            self.next_player,
            self.state,
            Default::default(),
            self.komi_2,
        )
    }

    /// Full zobrist, including:
    /// * the tiles
    /// * the next player
    /// * the pass state
    pub fn zobrist_full(&self) -> Zobrist {
        // TODO include rules?
        let mut result = self.chains().zobrist();
        result ^= Zobrist::for_color_turn(self.next_player);
        result ^= Zobrist::for_pass_state(self.state);
        result
    }

    pub fn random_available_place_move(&self, rng: &mut impl Rng) -> Result<Option<Move>, BoardDone> {
        self.check_done()?;

        let tile = self.chains.random_empty_tile_where(rng, |tile| {
            let mv = Move::Place(tile.to_tile(self.size()));
            self.is_available_move(mv).unwrap()
        });
        let mv = tile.map(|tile| Move::Place(tile.to_tile(self.size())));

        Ok(mv)
    }
}

fn is_available_move_sim(rules: &Rules, history: &IntSet<Zobrist>, kind: PlacementKind, next_zobrist: Zobrist) -> bool {
    // check placement kind
    match kind {
        PlacementKind::Normal => {}
        PlacementKind::Capture => {}
        PlacementKind::SuicideSingle => return false,
        PlacementKind::SuicideMulti => {
            if !rules.allow_multi_stone_suicide {
                return false;
            }
        }
    }

    // check history
    //   scan in reverse to hopefully find quicker matches
    if !rules.allow_repeating_tiles() && history.contains(&next_zobrist) {
        return false;
    }

    true
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
                    let tile = tile.to_flat(self.size());
                    match self.chains.simulate_place_stone_minimal(tile, self.next_player) {
                        Ok(sim) => is_available_move_sim(&self.rules, &self.history, sim.kind, sim.next_zobrist),
                        Err(TileOccupied) => false,
                    }
                }
            }
        };

        Ok(result)
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        // usually we'd check if the move is available too, but here we do that later
        self.check_done()?;

        let curr = self.next_player;
        let other = curr.other();

        match mv {
            Move::Pass => {
                // pass is always available
                // pass doesn't create history values or care about them

                // auxiliary state update
                self.next_player = other;
                self.state = match self.state {
                    State::Normal => State::Passed,
                    State::Passed => State::Done(self.current_score().to_outcome(self.komi_2)),
                    State::Done(_) => unreachable!(),
                };
            }
            Move::Place(tile) => {
                let prev_zobrist = self.chains.zobrist();

                // place the tile if the corresponding move is actually available, return error otherwise
                {
                    let tile = tile.to_flat(self.size());
                    let rules = &self.rules;
                    let history = &self.history;
                    let place_result = self.chains.place_stone_if(tile, curr, |sim| {
                        is_available_move_sim(rules, history, sim.kind, sim.next_zobrist)
                    });
                    match place_result {
                        Ok((_, true)) => {}
                        Ok((_, false)) => return Err(PlayError::UnavailableMove),
                        Err(TileOccupied) => return Err(PlayError::UnavailableMove),
                    }
                }

                // update history
                if self.rules.needs_history() {
                    self.history.insert(prev_zobrist);
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
        for tile in Tile::all(GO_MAX_SIZE) {
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
    // TODO include history (or just len?)
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zobrist_full().hash(state);
    }
}
