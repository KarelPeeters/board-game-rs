use std::fmt::{Debug, Display, Formatter, Write};

use itertools::Itertools;

use crate::board::Player;
use crate::games::ataxx::{AtaxxBoard, Coord, Tiles};

#[derive(Debug, Clone)]
pub struct InvalidAtaxxFen {
    pub fen: String,
    pub reason: &'static str,
}

impl AtaxxBoard {
    pub fn from_fen(fen: &str) -> Result<AtaxxBoard, InvalidAtaxxFen> {
        let err = |reason| InvalidAtaxxFen {
            fen: fen.into(),
            reason,
        };

        let blocks = fen.split(' ').collect_vec();
        let [board_str, next_str, half_str, full_str] = match &*blocks {
            &[a, b, c, d] => [a, b, c, d],
            _ => return Err(err("Not all 4 components present")),
        };

        // figure out the size, then parse the tiles and gaps
        let mut board = if board_str == "/" {
            AtaxxBoard::empty(0)
        } else {
            let rows = board_str.split('/').collect_vec();

            let size = rows.len();
            if size > AtaxxBoard::MAX_SIZE as usize {
                return Err(err("More rows than maximum board size"));
            }

            let mut board = AtaxxBoard::empty(size as u8);
            for (y, &line) in rows.iter().rev().enumerate().rev() {
                let mut x = 0;

                for c in line.chars() {
                    if x >= size {
                        return Err(err("Too many columns for size"));
                    }

                    let tile = Tiles::coord(Coord::from_xy(x as u8, y as u8));

                    match c {
                        'x' => board.tiles_a |= tile,
                        'o' => board.tiles_b |= tile,
                        '-' => board.gaps |= tile,
                        d if d.is_ascii_digit() => {
                            x += d.to_digit(10).unwrap() as usize;
                            continue;
                        }
                        _ => return Err(err("Invalid character in board")),
                    }

                    x += 1;
                }
            }

            board
        };

        // parse other details
        board.next_player = match next_str {
            "x" => Player::A,
            "o" => Player::B,
            _ => return Err(err("Invalid next player")),
        };

        board.moves_since_last_copy = half_str.parse::<u8>().map_err(|_| err("Invalid half counter"))?;
        let _ = full_str.parse::<u32>().map_err(|_| err("Invalid full counter"))?;

        board.update_outcome();
        board.assert_valid();

        Ok(board)
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();

        for y in (0..self.size).rev() {
            if y != self.size - 1 {
                write!(&mut s, "/").unwrap();
            }

            let mut empty_count = 0;

            for x in 0..self.size {
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

        write!(
            &mut s,
            " {} {} 1",
            player_symbol(self.next_player),
            self.moves_since_last_copy
        )
        .unwrap();

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

        for y in (0..self.size).rev() {
            write!(f, "{} ", y + 1)?;

            for x in 0..self.size {
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
                write!(
                    f,
                    "    {}  {}",
                    player_symbol(self.next_player),
                    self.moves_since_last_copy
                )?;
            }
            writeln!(f)?;
        }
        write!(f, "  ")?;
        for x in 0..self.size {
            write!(f, "{}", (b'a' + x) as char)?;
        }
        writeln!(f)?;

        Ok(())
    }
}

fn player_symbol(player: Player) -> char {
    match player {
        Player::A => 'x',
        Player::B => 'o',
    }
}
