use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use itertools::Itertools;
use static_assertions::const_assert;

use crate::board::{Board, Player};
use crate::games::go::chains::Chains;
use crate::games::go::tile::Tile;
use crate::games::go::{GoBoard, Move, PlacementKind, Rules, State, TileOccupied, GO_MAX_SIZE};

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Tile::x_to_char(self.x()).unwrap(), self.y() + 1)
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidTile;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidX;

const_assert!(GO_MAX_SIZE as usize <= Tile::TILE_X_NAMES.len());

impl Tile {
    // By convention 'I' is skipped because it can be confused with "1".
    pub const TILE_X_NAMES: &'static [u8] = b"ABCDEFGHJKLMNOPQRSTUVWXYZ";

    pub fn x_to_char(x: u8) -> Result<char, InvalidX> {
        Self::TILE_X_NAMES.get(x as usize).map(|&c| c as char).ok_or(InvalidX)
    }

    pub fn x_from_char(n: char) -> Result<u8, InvalidX> {
        Self::TILE_X_NAMES
            .iter()
            .position(|&c| c == n as u8)
            .map(|x| x as u8)
            .ok_or(InvalidX)
    }
}

impl FromStr for Tile {
    type Err = InvalidTile;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check(s.len() >= 2 && s.is_ascii(), InvalidTile)?;

        let x = Tile::x_from_char(s.as_bytes()[0] as char).map_err(|_| InvalidTile)?;
        let y_1 = s[1..].parse::<u8>().map_err(|_| InvalidTile)?;
        check(y_1 > 0, InvalidTile)?;

        Ok(Tile::new(x, y_1 - 1))
    }
}

impl Debug for GoBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GoBoard(fen={:?}, rules={:?}, history_len={})",
            self.to_fen(),
            self.rules(),
            self.history().len()
        )
    }
}

impl Display for GoBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let size = self.size();
        writeln!(f, "{:?}", self)?;

        for y in (0..size).rev() {
            write!(f, "{:2} ", y + 1)?;

            for x in 0..size {
                let tile = Tile::new(x, y);
                let tile_flat = tile.to_flat(size);
                let player = self.stone_at(tile);
                let c = match player {
                    None => {
                        let reaches_a = self.chains().reaches(tile_flat, Some(Player::A));
                        let reaches_b = self.chains().reaches(tile_flat, Some(Player::B));
                        if reaches_a ^ reaches_b {
                            '-'
                        } else {
                            '.'
                        }
                    }
                    Some(player) => player_symbol(player),
                };
                write!(f, "{}", c)?;
            }

            if y == size / 2 {
                write!(
                    f,
                    "    {}  {:?}  {:?}",
                    player_symbol(self.next_player()),
                    self.state(),
                    self.current_score()
                )?;
            }

            writeln!(f)?;
        }

        write!(f, "   ")?;
        for x in 0..size {
            write!(f, "{}", Tile::x_to_char(x).unwrap())?;
        }
        writeln!(f)?;

        Ok(())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Pass => write!(f, "PASS"),
            Move::Place(tile) => write!(f, "{}", tile),
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidMove;

impl FromStr for Move {
    type Err = InvalidMove;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "PASS" {
            Ok(Move::Pass)
        } else {
            match Tile::from_str(s) {
                Ok(tile) => Ok(Move::Place(tile)),
                Err(InvalidTile) => Err(InvalidMove),
            }
        }
    }
}

fn player_symbol(player: Player) -> char {
    match player {
        Player::A => 'b',
        Player::B => 'w',
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidFen {
    Syntax,
    InvalidChar,
    TooLarge,
    InvalidShape,
    HasDeadStones,
}

impl GoBoard {
    pub fn to_fen(&self) -> String {
        let chains = self.chains().to_fen();
        let next_player = player_symbol(self.next_player());
        let pass_counter = match self.state() {
            State::Normal => 0,
            State::Passed => 1,
            State::Done(_) => 2,
        };
        format!("{} {} {}", chains, next_player, pass_counter)
    }

    pub fn from_fen(fen: &str, rules: Rules) -> Result<GoBoard, InvalidFen> {
        let (tiles, next, pass) = fen.split(' ').collect_tuple().ok_or(InvalidFen::Syntax)?;

        let chains = Chains::from_fen(tiles)?;

        let next_player = match next {
            "b" => Player::A,
            "w" => Player::B,
            _ => return Err(InvalidFen::InvalidChar),
        };

        let state = match pass {
            "0" => State::Normal,
            "1" => State::Passed,
            "2" => State::Done(chains.score().to_outcome()),
            _ => return Err(InvalidFen::InvalidChar),
        };

        Ok(GoBoard::from_parts(
            rules,
            chains,
            next_player,
            state,
            Default::default(),
        ))
    }
}

impl Chains {
    pub fn to_fen(&self) -> String {
        let size = self.size();
        let mut fen = String::new();

        if size == 0 {
            fen.push('/');
        } else {
            for y in (0..size).rev() {
                for x in 0..size {
                    let tile = Tile::new(x, y).to_flat(size);
                    let player = self.stone_at(tile);
                    let c = match player {
                        None => '.',
                        Some(player) => player_symbol(player),
                    };
                    fen.push(c);
                }
                if y != 0 {
                    fen.push('/');
                }
            }
        }

        fen
    }

    pub fn from_fen(fen: &str) -> Result<Chains, InvalidFen> {
        check(fen.chars().all(|c| "/wb.".contains(c)), InvalidFen::InvalidChar)?;

        if fen == "/" {
            Ok(Chains::new(0))
        } else {
            let lines: Vec<&str> = fen.split('/').collect_vec();
            let size = lines.len();

            check(size <= GO_MAX_SIZE as usize, InvalidFen::TooLarge)?;
            let size = size as u8;

            let mut chains = Chains::new(size);
            for (y_rev, line) in lines.iter().enumerate() {
                let y = size as usize - 1 - y_rev;
                check(line.len() == size as usize, InvalidFen::InvalidShape)?;

                for (x, value) in line.chars().enumerate() {
                    let tile = Tile::new(x as u8, y as u8).to_flat(size);
                    let value = match value {
                        'b' => Some(Player::A),
                        'w' => Some(Player::B),
                        '.' => None,
                        _ => unreachable!(),
                    };

                    if let Some(player) = value {
                        let result = chains.place_stone(tile, player);
                        match result {
                            Ok(kind) => check(kind == PlacementKind::Normal, InvalidFen::HasDeadStones)?,
                            Err(TileOccupied) => unreachable!(),
                        }
                    }
                }
            }

            Ok(chains)
        }
    }
}

fn check<E>(c: bool, e: E) -> Result<(), E> {
    match c {
        true => Ok(()),
        false => Err(e),
    }
}
