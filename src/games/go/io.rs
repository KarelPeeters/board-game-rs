use std::fmt::{Alignment, Debug, Display, Formatter};
use std::str::FromStr;

use itertools::Itertools;

use crate::board::{Board, Player};
use crate::games::go::chains::Chains;
use crate::games::go::tile::Tile;
use crate::games::go::{GoBoard, Move, PlacementKind, Rules, State, TileOccupied, GO_MAX_SIZE};

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO support padding here?
        write!(f, "{}{}", self.x_disp(), self.y() as u32 + 1)
    }
}

impl Debug for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tile(({}, {}), {})", self.x(), self.y(), self)
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidTile;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidX;

// By convention 'I' is skipped because it can be confused with "1".
const TILE_X_NAMES_SINGLE: &[u8] = b"ABCDEFGHJKLMNOPQRSTUVWXYZ";

impl Tile {
    pub fn x_disp(&self) -> TileX {
        TileX(self.x())
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct TileX(pub u8);

impl FromStr for TileX {
    type Err = InvalidX;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn byte_index(c: u8) -> Result<usize, InvalidX> {
            TILE_X_NAMES_SINGLE
                .iter()
                .position(|&cand| cand == c.to_ascii_uppercase())
                .ok_or(InvalidX)
        }

        let x = match *s.as_bytes() {
            [c] => byte_index(c)?,
            [c1, c0] => (1 + byte_index(c1)?) * TILE_X_NAMES_SINGLE.len() + byte_index(c0)?,
            _ => return Err(InvalidX),
        };

        if x <= GO_MAX_SIZE as usize {
            Ok(TileX(x as u8))
        } else {
            Err(InvalidX)
        }
    }
}

impl Display for TileX {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let width = f.width().unwrap_or(0);

        let left = match f.align() {
            Some(Alignment::Left) => true,
            Some(Alignment::Center | Alignment::Right) | None => false,
        };

        let x = self.0 as usize;
        match TILE_X_NAMES_SINGLE.get(x).copied() {
            Some(b) => match left {
                true => write!(f, "{:<width$}", b as char, width = width),
                false => write!(f, "{:>width$}", b as char, width = width),
            },
            None => {
                let b1 = TILE_X_NAMES_SINGLE[(x / TILE_X_NAMES_SINGLE.len()) - 1] as char;
                let b0 = (TILE_X_NAMES_SINGLE[x % TILE_X_NAMES_SINGLE.len()] as char).to_ascii_lowercase();
                let pad = width.saturating_sub(2);
                match left {
                    true => write!(f, "{}{}{:pad$}", b1, b0, "", pad = pad),
                    false => write!(f, "{:pad$}{}{}", "", b1, b0, pad = pad),
                }
            }
        }
    }
}

impl FromStr for Tile {
    type Err = InvalidTile;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check(s.len() >= 2 && s.is_ascii(), InvalidTile)?;
        let split = s.bytes().take_while(|c| c.is_ascii_alphabetic()).count();

        let x = TileX::from_str(&s[..split]).map_err(|_| InvalidTile)?.0;

        let y_1 = s[split..].parse::<u32>().map_err(|_| InvalidTile)?;
        check(y_1 > 0, InvalidTile)?;
        let y = y_1 - 1;
        check(y <= GO_MAX_SIZE as u32, InvalidTile)?;
        let y = y as u8;

        Ok(Tile::new(x, y))
    }
}

impl GoBoard {
    fn write_debug(&self, f: &mut Formatter, include_fen: bool) -> std::fmt::Result {
        let next = match self.next_player() {
            Player::A => 'b',
            Player::B => 'w',
        };
        let fen = match include_fen {
            true => format!(", fen={:?}", self.to_fen()),
            false => String::new(),
        };

        write!(
            f,
            "GoBoard(next={:?}, state={:?}, history_len={}, stones_b={}, stones_w={}, rules={:?}{})",
            next,
            self.state(),
            self.history().len(),
            self.chains().stone_count_from(Player::A),
            self.chains().stone_count_from(Player::B),
            self.rules(),
            fen,
        )
    }
}

impl Debug for GoBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.write_debug(f, true)
    }
}

impl Display for GoBoard {
    // TODO re-introduce score and territory once we have optimized implementations for those?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.write_debug(f, false)?;
        writeln!(f)?;

        let size = self.size();
        let width_x = TileX(size.saturating_sub(1)).to_string().len();
        let width_y = size.to_string().len();

        for y in (0..size).rev() {
            write!(f, "{:width$} ", y + 1, width = width_y)?;

            for x in 0..size {
                let tile = Tile::new(x, y);
                let player = self.stone_at(tile);
                let c = match player {
                    None => '.',
                    Some(player) => player_symbol(player),
                };
                write!(f, "{:width$}", c, width = width_x)?;
            }

            writeln!(f)?;
        }

        write!(f, "{:width$}", "", width = width_y + 1)?;
        for x in 0..size {
            write!(f, "{:<width$}", TileX(x), width = width_x)?;
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
    Komi,
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
        let komi = self.komi_2() as f32 / 2.0;
        if komi == 0.0 {
            format!("{} {} {}", chains, next_player, pass_counter)
        } else {
            format!("{} {} {} {}", chains, next_player, pass_counter, komi)
        }
    }

    /// The fen format:
    /// `"tiles next pass [komi]"`
    pub fn from_fen(fen: &str, rules: Rules) -> Result<GoBoard, InvalidFen> {
        let values = fen.split(' ').collect_vec();
        let (&tiles, &next, &pass, komi) = match values.as_slice() {
            [tiles, next, pass] => (tiles, next, pass, None),
            [tiles, next, pass, komi] => (tiles, next, pass, Some(komi)),
            _ => return Err(InvalidFen::Syntax),
        };

        let chains = Chains::from_fen(tiles)?;

        let next_player = match next {
            "b" => Player::A,
            "w" => Player::B,
            _ => return Err(InvalidFen::InvalidChar),
        };

        let komi_2 = match komi {
            None => 0,
            Some(komi) => {
                let komi_f = komi.parse::<f32>().map_err(|_| InvalidFen::Komi)?;
                let komi_2f = komi_f * 2.0;
                let komi_2 = komi_2f as i16;
                if komi_2 as f32 != komi_2f {
                    return Err(InvalidFen::Komi);
                };
                komi_2
            }
        };

        let state = match pass {
            "0" => State::Normal,
            "1" => State::Passed,
            "2" => State::Done(chains.score().to_outcome(komi_2)),
            _ => return Err(InvalidFen::InvalidChar),
        };

        Ok(GoBoard::from_parts(
            rules,
            chains,
            next_player,
            state,
            Default::default(),
            komi_2,
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
                            Ok(sim) => check(sim.kind == PlacementKind::Normal, InvalidFen::HasDeadStones)?,
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
