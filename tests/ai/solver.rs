use board_game::ai::solver::is_double_forced_draw;
use board_game::board::{Board, BoardAvailableMoves, Outcome, Player};
use board_game::symmetry::UnitSymmetry;
use internal_iterator::{Internal, IteratorExt};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Tree {
    Outcome(Outcome),
    Node(Vec<Tree>),
}

impl Tree {
    fn choose(&mut self, i: usize) {
        if let Tree::Node(boards) = self {
            *self = boards.swap_remove(i);
        } else {
            panic!()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct DummyBoard {
    state: Tree,
    player: Player,
}

impl DummyBoard {
    fn new(state: Tree) -> Self {
        DummyBoard {
            state,
            player: Player::A,
        }
    }
}

impl fmt::Display for DummyBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Board for DummyBoard {
    type Move = usize;
    type Symmetry = UnitSymmetry;

    fn can_lose_after_move() -> bool {
        true
    }

    fn next_player(&self) -> Player {
        self.player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        if let Tree::Node(boards) = &self.state {
            mv < boards.len()
        } else {
            false
        }
    }

    fn play(&mut self, mv: Self::Move) {
        self.state.choose(mv);
        self.player = self.player.other();
    }

    fn outcome(&self) -> Option<Outcome> {
        match self.state {
            Tree::Node(_) => None,
            Tree::Outcome(outcome) => Some(outcome),
        }
    }

    fn map(&self, _sym: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(_sym: Self::Symmetry, mv: Self::Move) -> Self::Move {
        mv
    }
}

impl<'a> BoardAvailableMoves<'a, DummyBoard> for DummyBoard {
    type AllMoveIterator = Internal<std::ops::Range<usize>>;
    type MoveIterator = Internal<std::ops::Range<usize>>;

    fn all_possible_moves() -> Self::AllMoveIterator {
        (0..10).into_internal()
    }

    fn available_moves(&'a self) -> Self::MoveIterator {
        if let Tree::Node(boards) = &self.state {
            (0..boards.len()).into_internal()
        } else {
            (0..0).into_internal()
        }
    }
}

#[cfg(test)]
mod is_double_forced_draw {
    use super::*;

    #[test]
    fn draw0() {
        let draw = DummyBoard::new(Tree::Outcome(Outcome::Draw));
        assert_eq!(is_double_forced_draw(&draw, 0), Some(true));
        assert_eq!(is_double_forced_draw(&draw, 1), Some(true));
        assert_eq!(is_double_forced_draw(&draw, 2), Some(true));
        assert_eq!(is_double_forced_draw(&draw, 3), Some(true));
    }

    #[test]
    fn won0() {
        let won = DummyBoard::new(Tree::Outcome(Outcome::WonBy(Player::A)));
        assert_eq!(is_double_forced_draw(&won, 0), Some(false));
        assert_eq!(is_double_forced_draw(&won, 1), Some(false));
        assert_eq!(is_double_forced_draw(&won, 2), Some(false));
        assert_eq!(is_double_forced_draw(&won, 3), Some(false));
    }

    #[test]
    fn draw1() {
        let board = DummyBoard::new(Tree::Node(vec![Tree::Outcome(Outcome::Draw)]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(true));
        assert_eq!(is_double_forced_draw(&board, 2), Some(true));
        assert_eq!(is_double_forced_draw(&board, 3), Some(true));
    }

    #[test]
    fn won1() {
        let board = DummyBoard::new(Tree::Node(vec![Tree::Outcome(Outcome::WonBy(Player::A))]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(false));
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn draw2() {
        let board = DummyBoard::new(Tree::Node(vec![Tree::Node(vec![Tree::Outcome(Outcome::Draw)])]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(true));
        assert_eq!(is_double_forced_draw(&board, 3), Some(true));
    }

    #[test]
    fn won2() {
        let board = DummyBoard::new(Tree::Node(vec![Tree::Node(vec![Tree::Outcome(Outcome::WonBy(
            Player::A,
        ))])]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn draw1_draw1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::Draw),
            Tree::Outcome(Outcome::Draw),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(true));
        assert_eq!(is_double_forced_draw(&board, 2), Some(true));
        assert_eq!(is_double_forced_draw(&board, 3), Some(true));
    }

    #[test]
    fn won1_draw1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::WonBy(Player::A)),
            Tree::Outcome(Outcome::Draw),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(false));
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn draw1_won1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::Draw),
            Tree::Outcome(Outcome::WonBy(Player::A)),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(false));
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn draw1_draw2() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::Draw),
            Tree::Node(vec![Tree::Outcome(Outcome::Draw)]),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(true));
        assert_eq!(is_double_forced_draw(&board, 3), Some(true));
    }

    #[test]
    fn draw2_draw1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Node(vec![Tree::Outcome(Outcome::Draw)]),
            Tree::Outcome(Outcome::Draw),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(true));
        assert_eq!(is_double_forced_draw(&board, 3), Some(true));
    }

    #[test]
    fn draw1_won2() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::Draw),
            Tree::Node(vec![Tree::Outcome(Outcome::WonBy(Player::A))]),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn won2_draw1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Node(vec![Tree::Outcome(Outcome::WonBy(Player::A))]),
            Tree::Outcome(Outcome::Draw),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), None);
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn won1_draw2() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Outcome(Outcome::WonBy(Player::A)),
            Tree::Node(vec![Tree::Outcome(Outcome::Draw)]),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(false));
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }

    #[test]
    fn draw2_won1() {
        let board = DummyBoard::new(Tree::Node(vec![
            Tree::Node(vec![Tree::Outcome(Outcome::Draw)]),
            Tree::Outcome(Outcome::WonBy(Player::A)),
        ]));
        assert_eq!(is_double_forced_draw(&board, 0), None);
        assert_eq!(is_double_forced_draw(&board, 1), Some(false));
        assert_eq!(is_double_forced_draw(&board, 2), Some(false));
        assert_eq!(is_double_forced_draw(&board, 3), Some(false));
    }
}
