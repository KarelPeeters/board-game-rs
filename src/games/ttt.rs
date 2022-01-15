use std::fmt::{Debug, Display, Formatter};
use std::iter::Map;
use std::ops::Range;

use internal_iterator::{Internal, IteratorExt};

use crate::board::{Board, BoardAvailableMoves, BruteforceMoveIterator, Outcome, Player};
use crate::symmetry::UnitSymmetry;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Coord(usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TTTBoard {
    tiles: [Option<Player>; 9],
    next_player: Player,
    outcome: Option<Outcome>,
}

const LINES: &[[(usize, usize); 3]] = &[
    [(0, 0), (0, 1), (0, 2)],
    [(1, 0), (1, 1), (1, 2)],
    [(2, 0), (2, 1), (2, 2)],
    [(0, 0), (1, 0), (2, 0)],
    [(0, 1), (1, 1), (2, 1)],
    [(0, 2), (1, 2), (2, 2)],
    [(0, 0), (1, 1), (2, 2)],
    [(1, 2), (1, 1), (2, 0)],
];

impl Default for TTTBoard {
    fn default() -> Self {
        TTTBoard {
            tiles: Default::default(),
            next_player: Player::A,
            outcome: None,
        }
    }
}

impl Coord {
    pub fn from_xy(x: usize, y: usize) -> Self {
        assert!(x < 3);
        assert!(y < 3);
        Coord(y * 3 + x)
    }

    pub fn from_i(i: usize) -> Self {
        assert!(i < 9);
        Coord(i)
    }

    pub fn all() -> Map<Range<usize>, fn(usize) -> Coord> {
        let f: fn(usize) -> Coord = Coord;
        (0..9).map(f)
    }

    pub fn i(self) -> usize {
        self.0
    }

    pub fn x(self) -> usize {
        self.0 % 3
    }

    pub fn y(self) -> usize {
        self.0 / 3
    }
}

impl TTTBoard {
    pub fn tile(&self, coord: Coord) -> Option<Player> {
        self.tiles[coord.0]
    }
}

impl Board for TTTBoard {
    type Move = Coord;
    type Symmetry = UnitSymmetry;

    fn can_lose_after_move() -> bool {
        false
    }

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());
        self.tiles[mv.0] == None
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(!self.is_done());
        assert!(self.is_available_move(mv));

        self.tiles[mv.0] = Some(self.next_player);

        let won = LINES.iter().any(|line| {
            line.iter()
                .all(|&(lx, ly)| self.tiles[Coord::from_xy(lx, ly).0] == Some(self.next_player))
        });
        let draw = self.tiles.iter().all(|tile| tile.is_some());

        self.outcome = if won {
            Some(Outcome::WonBy(self.next_player))
        } else if draw {
            Some(Outcome::Draw)
        } else {
            None
        };

        self.next_player = self.next_player.other();
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn map(&self, _: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(&self, _: Self::Symmetry, mv: Self::Move) -> Self::Move {
        mv
    }
}

impl<'a> BoardAvailableMoves<'a, TTTBoard> for TTTBoard {
    type AllMoveIterator = Internal<Map<Range<usize>, fn(usize) -> Coord>>;
    type MoveIterator = BruteforceMoveIterator<'a, TTTBoard>;

    fn all_possible_moves() -> Self::AllMoveIterator {
        Coord::all().into_internal()
    }

    fn available_moves(&'a self) -> Self::MoveIterator {
        assert!(!self.is_done());
        BruteforceMoveIterator::new(self)
    }
}

fn tile_to_char(tile: Option<Player>) -> char {
    match tile {
        Some(Player::A) => 'a',
        Some(Player::B) => 'b',
        None => ' ',
    }
}

impl Debug for Coord {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Coord({}, {})", self.x(), self.y())
    }
}

impl Display for Coord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x(), self.y())
    }
}

impl Display for TTTBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "+---+")?;
        for y in 0..3 {
            write!(f, "|")?;
            for x in 0..3 {
                let coord = Coord::from_xy(x, y);
                write!(f, "{}", tile_to_char(self.tiles[coord.0]))?;
            }
            write!(f, "|")?;

            if y == 1 {
                write!(f, "   {}", tile_to_char(Some(self.next_player)))?;
            }

            writeln!(f)?;
        }

        writeln!(f, "+---+")?;
        Ok(())
    }
}
