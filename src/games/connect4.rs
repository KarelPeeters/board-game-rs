//! Bitboard implementation based on http://blog.gamesolver.org/solving-connect-four/06-bitboard/.

use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;

use internal_iterator::{Internal, IteratorExt};

use crate::board::{Board, BoardMoves, BoardSymmetry, BruteforceMoveIterator, Outcome, Player};
use crate::symmetry::D1Symmetry;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Connect4 {
    tiles_next: u64,
    tiles_occupied: u64,
    next_player: Player,
    outcome: Option<Outcome>,
}

impl Connect4 {
    pub const WIDTH: u8 = 7;
    pub const HEIGHT: u8 = 6;
}

impl Default for Connect4 {
    fn default() -> Self {
        Connect4 {
            tiles_next: 0,
            tiles_occupied: 0,
            next_player: Player::A,
            outcome: None,
        }
    }
}

impl Board for Connect4 {
    type Move = u8;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());
        assert!(mv < Self::WIDTH);
        self.tiles_occupied & mask(mv, Self::HEIGHT - 1) == 0
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(self.is_available_move(mv));

        // play move
        self.tiles_next ^= self.tiles_occupied;
        self.tiles_occupied |= self.tiles_occupied + mask(mv, 0);

        //update outcome
        const TOP_ROW: u64 = 0x20202020202020;
        if self.tiles_occupied & TOP_ROW == TOP_ROW {
            self.outcome = Some(Outcome::Draw);
        } else {
            let tiles_curr = self.tiles_next ^ self.tiles_occupied;
            for half in [1, 9, 8, 7] {
                let m0 = tiles_curr & (tiles_curr << half);
                let m1 = m0 & (m0 << (half * 2));
                if m1 != 0 {
                    self.outcome = Some(Outcome::WonBy(self.next_player));
                    break;
                }
            }
        }

        self.next_player = self.next_player.other();
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn can_lose_after_move() -> bool {
        false
    }
}

impl<'a> BoardMoves<'a, Connect4> for Connect4 {
    type AllMovesIterator = Internal<Range<u8>>;
    type AvailableMovesIterator = BruteforceMoveIterator<'a, Connect4>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        (0..Self::WIDTH).into_internal()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
        BruteforceMoveIterator::new(self)
    }
}

impl BoardSymmetry<Connect4> for Connect4 {
    type Symmetry = D1Symmetry;

    fn map(&self, sym: Self::Symmetry) -> Self {
        if sym.mirror {
            Connect4 {
                tiles_next: self.tiles_next.swap_bytes(),
                tiles_occupied: self.tiles_occupied.swap_bytes(),
                next_player: self.next_player,
                outcome: self.outcome,
            }
        } else {
            self.clone()
        }
    }

    fn map_move(&self, sym: Self::Symmetry, mv: u8) -> u8 {
        assert!(mv < Self::WIDTH);
        if sym.mirror {
            Self::WIDTH - mv - 1
        } else {
            mv
        }
    }
}

impl Debug for Connect4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (sep, line) = if f.alternate() { ("\n    ", "\n") } else { (" ", "") };

        write!(
            f,
            "Connect4 {{{}tiles_next: {:x},{}tiles_occupied: {:x},{}next_player: {:?},{}outcome: {:?}{}}}",
            sep, self.tiles_next, sep, self.tiles_occupied, sep, self.next_player, sep, self.outcome, line,
        )
    }
}

impl Display for Connect4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (tiles_a, tiles_b) = match self.next_player {
            Player::A => (self.tiles_next, self.tiles_next ^ self.tiles_occupied),
            Player::B => (self.tiles_next ^ self.tiles_occupied, self.tiles_next),
        };

        for row in (0..Self::HEIGHT).rev() {
            for col in 0..Self::WIDTH {
                let c = match (get(tiles_a, col, row), get(tiles_b, col, row)) {
                    (true, false) => 'a',
                    (false, true) => 'b',
                    (false, false) => '.',
                    _ => unreachable!(),
                };

                write!(f, "{}", c)?;
            }
            if row == Self::HEIGHT / 2 {
                write!(f, "    {}", self.next_player.to_char())?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

fn mask(col: u8, row: u8) -> u64 {
    1 << (row + (col * 8))
}

fn get(tiles: u64, col: u8, row: u8) -> bool {
    tiles & mask(col, row) != 0
}
