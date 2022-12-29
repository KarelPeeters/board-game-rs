//! Oware is a strategy game from the family of mancala
//!
//! It has no  combos, i.e. one turn each round each player.
//! Official version contains 12 pits and 4 initial seeds in each pit. This implementation is const generic parameterized with PITS_PER_PLAYER.
//!
//! # Rules
//! 1. Each player has control over half the pits that are in front of them
//! 2. Player has to choose a non-empty pit. This removes the seeds from that pit and
//!    distributes them, dropping one in each pit, except the selected pit,
//!    in counter-clock wise.
//! 3. If the last seed is dropped in one of the opponents pit, and the final seed count in that
//!    pit is 2 or 3, then the current player captures it. It can happen several times if
//!    continous multiple pits to the last dropped pit also have `2` or `3` seeds in them.
//!    If all the seeds could be captured this turn, then no capture happens
//!
//! Objective is to capture as many seeds as possible and so the game will conclude early if a
//! player captures more than half the seeds
use internal_iterator::{Internal, IteratorExt};
use itertools::join;
use std::fmt::{Debug, Display, Formatter};

use crate::board::{Alternating, Board, BoardMoves, BruteforceMoveIterator, Outcome, Player};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct OwareBoard<const PITS_PER_PLAYER: usize> {
    pits: [[u8; PITS_PER_PLAYER]; 2],
    scores: [u8; 2],
    next_player: Player,
    outcome: Option<Outcome>,
    init_seeds: u8,
}

impl<const P: usize> Default for OwareBoard<P> {
    fn default() -> Self {
        Self::new(4)
    }
}

impl<const PITS: usize> OwareBoard<PITS> {
    pub fn new(init_seeds: u8) -> Self {
        Self {
            pits: [[init_seeds; PITS]; 2],
            init_seeds,
            scores: Default::default(),
            next_player: Player::A,
            outcome: None,
        }
    }

    pub fn score(&self, player: Player) -> u8 {
        self.scores[player.index() as usize]
    }

    pub fn get_seeds(&self, player: Player, idx: usize) -> u8 {
        self.pits[player.index() as usize][idx]
    }

    pub fn init_seeds(&self) -> u8 {
        self.init_seeds
    }

    pub fn pits(&self) -> [[u8; PITS]; 2] {
        self.pits
    }

    fn grand_slam(&self, mv: usize) -> bool {
        mv >= PITS
            && self.opp_pits().all(|x| match self.at(x) {
                2 | 3 => x <= mv,
                0 => x > mv,
                _ => false,
            })
            && self.pl_pits().any(|x| self.at(x) > 0)
    }

    fn pl_pits(&self) -> std::ops::Range<usize> {
        0..PITS
    }

    fn opp_pits(&self) -> std::ops::Range<usize> {
        PITS..PITS * 2
    }

    // gets the number of seeds w.r.t the `next_player` for a given index
    fn at(&self, idx: usize) -> u8 {
        self.pits[usize::from(idx >= PITS) ^ self.next_player.index() as usize][idx % PITS]
    }

    fn capture(&mut self, idx: usize) -> u8 {
        let seeds = self.pits[usize::from(idx >= PITS) ^ self.next_player.index() as usize][idx % PITS];
        self.pits[usize::from(idx >= PITS) ^ self.next_player.index() as usize][idx % PITS] = 0;
        seeds
    }

    fn can_overflow(&self, mv: usize) -> bool {
        mv % PITS + self.at(mv) as usize >= PITS
    }

    fn is_stalemate(&self) -> bool {
        if (0..PITS * 2).fold(0, |a, c| a + self.at(c)) == 2 && self.pits.iter().flatten().max() == Some(&1) {
            let f = (0..PITS * 2).position(|x| self.at(x) == 1).unwrap();
            let l = (0..PITS * 2).rposition(|x| self.at(x) == 1).unwrap();
            (PITS - 1..PITS + 1).contains(&(l - f))
        } else {
            false
        }
    }
}

impl<const PITS: usize> Board for OwareBoard<PITS> {
    type Move = usize;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> bool {
        assert!(!self.is_done());
        assert!(mv < PITS);
        if self.opp_pits().any(|x| self.at(x) > 0) {
            self.at(mv) > 0
        } else {
            self.can_overflow(mv)
        }
    }

    fn play(&mut self, mv: Self::Move) {
        assert!(self.is_available_move(mv), "{:?} is not available on {:?}", mv, self);

        let player = self.next_player.index() as usize;

        let mut seeds = self.capture(mv);
        let mut idx = mv;

        // sowing
        while seeds > 0 {
            idx = (idx + usize::from((idx + 1) % (PITS * 2) == mv) + 1) % (PITS * 2);
            seeds -= 1;
            self.pits[usize::from(idx >= PITS) ^ player][idx % PITS] += 1;
        }

        // capture
        if !self.grand_slam(idx) {
            while idx >= PITS && matches!(self.at(idx), 2 | 3) {
                self.scores[player] += self.capture(idx);
                idx = (idx + (PITS * 2) - 1) % (PITS * 2);
            }
        }

        // No move endgame
        if self.pl_pits().all(|x| self.at(x) == 0) && !self.opp_pits().any(|x| self.can_overflow(x)) {
            self.opp_pits().for_each(|x| {
                self.scores[(player + 1) % 2] += self.capture(x);
            })
        }

        // Stalemate endgame
        if self.is_stalemate() {
            (0..PITS * 2).for_each(|x| _ = self.capture(x));
            self.scores.iter_mut().for_each(|x| *x += 1);
        }

        assert!(
            self.pits.iter().flatten().chain(self.scores.iter()).sum::<u8>() == 2 * PITS as u8 * self.init_seeds,
            "{} seeds should exist",
            2 * PITS as u8 * self.init_seeds
        );

        let draw = self.scores.iter().all(|&x| x == PITS as u8 * self.init_seeds);

        self.outcome = self
            .scores
            .iter()
            .position(|&score| score > PITS as u8 * self.init_seeds)
            .map_or(if draw { Some(Outcome::Draw) } else { None }, |pl| {
                Some(Outcome::WonBy(Player::BOTH[pl]))
            });

        self.next_player = self.next_player.other();
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    fn can_lose_after_move() -> bool {
        false
    }
}

impl<const PITS: usize> Alternating for OwareBoard<PITS> {}

impl<const PITS: usize> crate::board::BoardSymmetry<OwareBoard<PITS>> for OwareBoard<PITS> {
    type Symmetry = crate::symmetry::UnitSymmetry;
    type CanonicalKey = ();

    fn map(&self, _: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(&self, _: Self::Symmetry, mv: <OwareBoard<PITS> as Board>::Move) -> <OwareBoard<PITS> as Board>::Move {
        mv
    }

    fn canonical_key(&self) -> Self::CanonicalKey {}
}

impl<'a, const PITS: usize> BoardMoves<'a, OwareBoard<PITS>> for OwareBoard<PITS> {
    type AllMovesIterator = Internal<std::ops::Range<usize>>;
    type AvailableMovesIterator = BruteforceMoveIterator<'a, OwareBoard<PITS>>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        (0..PITS).into_internal()
    }

    fn available_moves(&'a self) -> Self::AvailableMovesIterator {
        BruteforceMoveIterator::new(self)
    }
}

impl<const PITS: usize> Display for OwareBoard<PITS> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = format!("       {}    S   ", join(0..PITS, "    "));
        writeln!(f, "{}", s.chars().rev().collect::<String>())?;
        writeln!(f, "┌──{}──┐", "──┬──".repeat(PITS + 1))?;
        writeln!(
            f,
            "│ {:2} │ {} │ ←B │",
            self.scores[0],
            join((0..PITS).rev().map(|x| format!("{:2}", self.pits[1][x])), " │ "),
        )?;
        writeln!(f, "│    ├──{}──┤    │", "──┼──".repeat(PITS - 1))?;
        writeln!(
            f,
            "│ A→ │ {} │ {:2} │",
            join((0..PITS).map(|x| format!("{:2}", self.pits[0][x])), " │ "),
            self.scores[1],
        )?;
        writeln!(f, "└──{}──┘", "──┴──".repeat(PITS + 1))?;
        writeln!(f, "{s}")?;
        Ok(())
    }
}
