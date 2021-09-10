use std::fmt::{Debug, Display, Formatter, Write};

use regex::Regex;

use crate::board::Player;
use crate::games::ataxx::{AtaxxBoard, Coord, Tiles};

const FEN_REGEX: &str = r"(?x)(?-u)
    ^ ([ox\-\d]+)/([ox\-\d]+)/([ox\-\d]+)/([ox\-\d]+)/([ox\-\d]+)/([ox\-\d]+)/([ox\-\d]+)
    \s (?P<next>[ox]) \s (?P<half>\d+) \s (?P<full>\d+) $
";

impl AtaxxBoard {
    pub fn from_fen(fen: &str) -> AtaxxBoard {
        let mut board = AtaxxBoard::empty();

        let regex = Regex::new(FEN_REGEX).unwrap();
        let captures = regex.captures(fen)
            .unwrap_or_else(|| panic!("Invalid fen {:?}", fen));
        assert_eq!(1 + 7 + 3, captures.len());

        for y in (0..7).rev() {
            let line = &captures[7 - y];
            let mut x = 0;
            for c in line.chars() {
                if x >= 7 { panic!("Line {:?} too long", line) }
                let tiles = Tiles::coord(Coord::from_xy(x, y as u8));
                match c {
                    'x' => board.tiles_a |= tiles,
                    'o' => board.tiles_b |= tiles,
                    '-' => board.gaps |= tiles,
                    d if d.is_ascii_digit() => {
                        x += d.to_digit(10).unwrap() as u8;
                        continue;
                    }
                    _ => unreachable!(),
                }
                x += 1;
            }
            assert_eq!(x, 7, "Line {:?} too short", line);
        }

        board.next_player = match &captures["next"] {
            "x" => Player::A,
            "o" => Player::B,
            _ => unreachable!(),
        };
        board.moves_since_last_copy = captures["half"].parse::<u8>().unwrap();

        board.update_outcome();
        board
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();

        for y in (0..7).rev() {
            if y != 6 {
                write!(&mut s, "/").unwrap();
            }

            let mut empty_count = 0;

            for x in 0..7 {
                let coord = Coord::from_xy(x, y);

                if self.free_tiles().has(coord) {
                    empty_count += 1;
                } else {
                    if empty_count != 0 {
                        write!(&mut s, "{}", empty_count).unwrap();
                        empty_count = 0;
                    }

                    match self.tile(coord) {
                        Some(player) => {
                            write!(&mut s, "{}", player_symbol(player)).unwrap();
                        }
                        None => {
                            assert!(self.gaps.has(coord));
                            write!(&mut s, "-").unwrap();
                        }
                    }
                }
            }

            if empty_count != 0 {
                write!(&mut s, "{}", empty_count).unwrap();
            }
        }

        write!(&mut s, " {} {} 1", player_symbol(self.next_player), self.moves_since_last_copy).unwrap();

        s
    }
}

impl Debug for AtaxxBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AtaxxBoard(\"{}\")", self.to_fen())
    }
}

impl Display for AtaxxBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FEN: {}", self.to_fen())?;

        for y in (0..7).rev() {
            write!(f, "{} ", y + 1)?;

            for x in 0..7 {
                let coord = Coord::from_xy(x, y);
                let tuple = (self.gaps.has(coord), self.tile(coord));
                let c = match tuple {
                    (true, None) => '-',
                    (false, None) => '.',
                    (false, Some(player)) => player_symbol(player),
                    (true, Some(_)) => unreachable!("Tile with block cannot have player"),
                };

                write!(f, "{}", c)?;
            }

            if y == 3 {
                write!(f, "    {}  {}", player_symbol(self.next_player), self.moves_since_last_copy)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  abcdefg")?;

        Ok(())
    }
}

fn player_symbol(player: Player) -> char {
    match player {
        Player::A => 'x',
        Player::B => 'o',
    }
}