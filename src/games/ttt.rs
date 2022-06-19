use std::fmt::{Debug, Display, Formatter};

use internal_iterator::{Internal, IteratorExt};

use crate::board::{Alternating, Board, BoardMoves, BruteforceMoveIterator, Outcome, Player, UnitSymmetryBoard};
use crate::util::coord::{Coord3, CoordAllIter};

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
    [(0, 2), (1, 1), (2, 0)],
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

impl TTTBoard {
    pub fn tile(&self, coord: Coord3) -> Option<Player> {
        self.tiles[coord.index() as usize]
    }
}

impl Board for TTTBoard {
    type Move = Coord3;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());
        self.tiles[mv.index() as usize] == None
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(self.is_available_move(mv), "{:?} is not available on {:?}", mv, self);

        self.tiles[mv.index() as usize] = Some(self.next_player);

        let won = LINES.iter().any(|line| {
            line.iter().all(|&(lx, ly)| {
                let li = Coord3::from_xy(lx as u8, ly as u8).index() as usize;
                self.tiles[li] == Some(self.next_player)
            })
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

    fn can_lose_after_move() -> bool {
        false
    }
}

impl Alternating for TTTBoard {}

impl UnitSymmetryBoard for TTTBoard {}

impl<'a> BoardMoves<'a, TTTBoard> for TTTBoard {
    type AllMovesIterator = Internal<CoordAllIter<Coord3>>;
    type AvailableMovesIterator = BruteforceMoveIterator<'a, TTTBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        Coord3::all().into_internal()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
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

impl Display for TTTBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "+---+")?;
        for y in 0..3 {
            write!(f, "|")?;
            for x in 0..3 {
                let coord = Coord3::from_xy(x, y);
                write!(f, "{}", tile_to_char(self.tiles[coord.index() as usize]))?;
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
