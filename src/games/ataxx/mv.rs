use std::cmp::max;
use std::fmt::{Debug, Formatter};

use crate::games::ataxx::tiles::Tiles;
use crate::symmetry::D4Symmetry;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Coord(u8);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Move {
    Pass,
    Copy { to: Coord },
    Jump { from: Coord, to: Coord },
}

impl Debug for Coord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uai())
    }
}

impl Debug for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uai())
    }
}

impl Coord {
    pub fn all() -> impl Iterator<Item=Coord> {
        // this is kind of stupid but it works
        Tiles::full().into_iter()
    }

    pub fn from_xy(x: u8, y: u8) -> Coord {
        assert!(x < 7);
        assert!(y < 7);
        Coord(x + 8 * y)
    }

    pub fn from_sparse_i(i: u8) -> Coord {
        Coord::from_xy(i % 8, i / 8)
    }

    pub fn x(self) -> u8 {
        self.0 % 8
    }

    pub fn y(self) -> u8 {
        self.0 / 8
    }

    pub fn sparse_i(self) -> u8 {
        self.0
    }

    pub fn dense_i(self) -> u8 {
        self.x() + 7 * self.y()
    }

    pub fn distance(self, other: Coord) -> u8 {
        let dx = abs_distance(self.x(), other.x());
        let dy = abs_distance(self.y(), other.y());
        max(dx, dy)
    }

    pub fn map(self, sym: D4Symmetry) -> Coord {
        let (x, y) = sym.map_xy(self.x(), self.y(), 6);
        Coord::from_xy(x, y)
    }
}

fn abs_distance(a: u8, b: u8) -> u8 {
    if a >= b { a - b } else { b - a }
}

impl Coord {
    pub fn to_uai(self) -> String {
        format!("{}{}", ('a' as u8 + self.x()) as char, self.y() + 1)
    }

    pub fn from_uai(s: &str) -> Coord {
        assert_eq!(s.len(), 2);
        let x = s.as_bytes()[0] - b'a';
        let y = (s.as_bytes()[1] - b'0') - 1;
        Coord::from_xy(x, y)
    }
}

impl Move {
    pub fn to_uai(self) -> String {
        match self {
            Move::Pass => "0000".to_string(),
            Move::Copy { to } => to.to_uai(),
            Move::Jump { from, to } => format!("{}{}", from.to_uai(), to.to_uai())
        }
    }

    pub fn from_uai(s: &str) -> Move {
        match s {
            "0000" => Move::Pass,
            _ if s.len() == 2 => Move::Copy { to: Coord::from_uai(s) },
            _ if s.len() == 4 => Move::Jump { from: Coord::from_uai(&s[..2]), to: Coord::from_uai(&s[2..]) },
            _ => panic!("Invalid move uai string '{}'", s)
        }
    }
}
