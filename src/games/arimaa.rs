use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::ControlFlow;
use std::str::FromStr;

use arimaa_engine_step::{Action, Direction, GameState, Piece, Square, Terminal};
use internal_iterator::InternalIterator;
use once_cell::sync::OnceCell;

use crate::board::{AllMovesIterator, AvailableMovesIterator, Board, BoardMoves, Outcome, Player, UnitSymmetryBoard};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ArimaaBoard {
    state: GameState,

    // careful, this should be invalidated whenever the state is modified
    available_moves_cache: OnceCell<Vec<Action>>,
}

impl Default for ArimaaBoard {
    fn default() -> Self {
        ArimaaBoard::from_state(GameState::initial())
    }
}

impl ArimaaBoard {
    pub fn from_state(state: GameState) -> Self {
        ArimaaBoard {
            state,
            available_moves_cache: OnceCell::new(),
        }
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    fn init_available_moves(&self) -> &[Action] {
        self.available_moves_cache.get_or_init(|| self.state.valid_actions())
    }
}

impl Board for ArimaaBoard {
    type Move = Action;

    fn next_player(&self) -> Player {
        match self.state.is_p1_turn_to_move() {
            true => Player::A,
            false => Player::B,
        }
    }

    fn is_available_move(&self, mv: Action) -> bool {
        assert!(!self.is_done());
        self.init_available_moves().contains(&mv)
    }

    fn play(&mut self, mv: Action) {
        assert!(!self.is_done());
        assert!(self.is_available_move(mv));

        self.state = self.state.take_action(&mv);
        self.available_moves_cache = OnceCell::new();
    }

    fn outcome(&self) -> Option<Outcome> {
        self.state.is_terminal().map(|t| match t {
            Terminal::GoldWin => Outcome::WonBy(Player::A),
            Terminal::SilverWin => Outcome::WonBy(Player::B),
        })
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl<'a> BoardMoves<'a, ArimaaBoard> for ArimaaBoard {
    type AllMovesIterator = AllMovesIterator<ArimaaBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, ArimaaBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
        assert!(!self.is_done());
        AvailableMovesIterator(self)
    }
}

impl InternalIterator for AllMovesIterator<ArimaaBoard> {
    type Item = Action;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        f(Action::Pass)?;
        for piece in Piece::ALL {
            f(Action::Place(piece))?;
        }
        for square in 0..64 {
            let square = Square::from_index(square);
            for direction in Direction::ALL {
                f(Action::Move(square, direction))?;
            }
        }

        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, ArimaaBoard> {
    type Item = Action;

    fn try_for_each<R, F>(self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        self.0.init_available_moves().iter().copied().try_for_each(f)
    }
}

impl Hash for ArimaaBoard {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.state.transposition_hash())
    }
}

impl UnitSymmetryBoard for ArimaaBoard {}

impl Display for ArimaaBoard {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Display::fmt(&self.state, f)
    }
}

impl FromStr for ArimaaBoard {
    type Err = <GameState as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GameState::from_str(s).map(|state| ArimaaBoard::from_state(state))
    }
}
