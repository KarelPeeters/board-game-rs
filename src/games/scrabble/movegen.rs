use std::fmt::{Debug, Formatter};
use std::ops::ControlFlow;

use fst::raw::Node;

use crate::games::scrabble::basic::{Deck, Letter, Mask, MAX_DECK_SIZE};

pub type Set = fst::Set<Vec<u8>>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

// TODO this should be some generic stack vec instead
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Placed {
    inner: [Letter; MAX_DECK_SIZE],
    len: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Move {
    // TODO encode which tiles use a wildcard tile
    //   does it ever make sense to use a wildcard when not necessary?
    //   does it make sense to place the wildcard on a multiplier tile when possible to avoid?

    // TODO remove obsolete fields
    // TODO make this struct smaller with a bunch of bit fiddling?
    //  (or make a separate struct for that)
    pub dir: Direction,
    pub x: u8,
    pub y: u8,
    pub placed: Placed,

    pub forward_count: usize,
    pub backward_count: usize,

    pub score: u32,
}

#[derive(Debug)]
pub struct Cell {
    pub letter: Option<Letter>,
    pub attached: bool,
    pub allowed: Mask,

    pub score_cross: u32,
    pub word_multiplier: u8,
    pub letter_multiplier: u8,
}

impl Direction {
    pub const ALL: [Direction; 2] = [Direction::Horizontal, Direction::Vertical];

    pub fn delta(self) -> (u8, u8) {
        match self {
            Direction::Horizontal => (1, 0),
            Direction::Vertical => (0, 1),
        }
    }

    pub fn orthogonal(self) -> Self {
        match self {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Direction::Horizontal => 0,
            Direction::Vertical => 1,
        }
    }
}

impl Cell {
    fn is_anchor(&self) -> bool {
        self.letter.is_none() && self.attached
    }
}

struct MoveGen<'a, R, F: FnMut(Move) -> ControlFlow<R>> {
    set: &'a Set,
    dir: Direction,
    row: u8,
    cells: &'a [Cell],
    anchor: usize,
    f: F,
}

#[must_use]
pub fn movegen<R>(
    set: &Set,
    dir: Direction,
    row: u8,
    cells: &[Cell],
    deck: Deck,
    mut f: impl FnMut(Move) -> ControlFlow<R>,
) -> ControlFlow<R> {
    for curr in 0..cells.len() {
        let cell = &cells[curr];
        if !cell.is_anchor() {
            continue;
        }

        let mut movegen = MoveGen {
            set,
            dir,
            row,
            cells,
            anchor: curr,
            f: &mut f,
        };
        movegen.run(deck)?;
    }

    ControlFlow::Continue(())
}

#[derive(Debug, Copy, Clone)]
struct PartialScore {
    sum_orthogonal: u32,
    sum_word: u32,
    total_word_multiplier: u32,
    tiles_placed: u8,
}

impl Default for Placed {
    fn default() -> Self {
        Placed {
            inner: [Letter::from_char('a').unwrap(); MAX_DECK_SIZE],
            len: 0,
        }
    }
}

impl Placed {
    pub fn push_back(self, letter: Letter) -> Self {
        let index = self.len as usize;
        assert!(index < self.inner.len());
        let mut inner = self.inner;
        inner[index] = letter;
        Placed {
            inner,
            len: self.len + 1,
        }
    }

    pub fn insert_front(self, letter: Letter) -> Self {
        assert!((self.len as usize) < (self.inner.len()));

        let mut inner = self.inner;
        inner.copy_within(0..self.inner.len() - 1, 1);
        inner[0] = letter;

        Placed {
            inner,
            len: self.len + 1,
        }
    }

    pub fn len(self) -> u8 {
        self.len
    }

    pub fn iter(self) -> impl Iterator<Item = Letter> {
        (0..self.len).map(move |i| self[i])
    }
}

impl std::ops::Index<u8> for Placed {
    type Output = Letter;

    fn index(&self, index: u8) -> &Self::Output {
        assert!(index < self.len());
        &self.inner[index as usize]
    }
}

impl Debug for Placed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Default for PartialScore {
    fn default() -> Self {
        PartialScore {
            sum_word: 0,
            sum_orthogonal: 0,
            total_word_multiplier: 1,
            tiles_placed: 0,
        }
    }
}

impl PartialScore {
    fn add(mut self, placed: bool, letter: Letter, cell: &Cell) -> Self {
        let letter_mul = cell.letter_multiplier as u32;
        let word_mul = cell.word_multiplier as u32;

        if placed {
            debug_assert!(cell.letter.is_none());
            self.tiles_placed += 1;

            // both multipliers count
            let letter_value = letter.score_value() as u32 * letter_mul;
            self.sum_word += letter_value;
            self.total_word_multiplier *= word_mul;

            // add cross score if any
            if cell.score_cross != 0 {
                self.sum_orthogonal += (letter_value + cell.score_cross) * word_mul;
            }
        } else {
            // only count letter for word, non-multiplied
            self.sum_word += letter.score_value() as u32;
        }

        self
    }

    fn total(self) -> u32 {
        debug_assert!(
            0 < self.tiles_placed && self.tiles_placed <= 7,
            "Invalid tiles_placed: {}",
            self.tiles_placed
        );

        let bingo_bonus = if self.tiles_placed == 7 { 50 } else { 0 };
        self.sum_orthogonal + self.sum_word * self.total_word_multiplier + bingo_bonus
    }
}

impl<R, F: FnMut(Move) -> ControlFlow<R>> MoveGen<'_, R, F> {
    fn run(&mut self, deck: Deck) -> ControlFlow<R> {
        let root = self.set.as_fst().root();
        self.run_recurse_forward(deck, root, 0, Placed::default(), PartialScore::default())
    }

    #[must_use]
    fn run_recurse_forward(
        &mut self,
        mut deck: Deck,
        node: Node,
        forward_count: usize,
        placed: Placed,
        score: PartialScore,
    ) -> ControlFlow<R> {
        let cell = self.anchor + forward_count;

        // force direction change when hitting the right edge
        if cell >= self.cells.len() {
            if let Some(index) = node.find_input(b'+') {
                let trans = node.transition(index);
                let next = self.set.as_fst().node(trans.addr);
                self.run_recurse_backward(deck, next, placed, forward_count, 0, score)?;
            }

            return ControlFlow::Continue(());
        }
        let cell = &self.cells[cell];

        for trans in node.transitions() {
            let char = trans.inp;
            let next = self.set.as_fst().node(trans.addr);

            if char == b'+' {
                // switch direction if possible
                if cell.letter.is_none() {
                    self.run_recurse_backward(deck, next, placed, forward_count, 0, score)?;
                }
            } else {
                // place letter if possible
                let letter = Letter::from_char(char as char).unwrap();

                if cell.allowed.get(letter) {
                    // TODO clean this up a bit
                    let used_deck = if let Some(actual) = cell.letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    let new_placed = if used_deck { placed.push_back(letter) } else { placed };
                    let new_score = score.add(used_deck, letter, cell);

                    self.run_recurse_forward(deck, next, forward_count + 1, new_placed, new_score)?;

                    // TODO maybe just use the old deck instead of undoing changes
                    if used_deck {
                        deck.add(letter);
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[must_use]
    fn run_recurse_backward(
        &mut self,
        mut deck: Deck,
        node: Node,
        placed: Placed,
        forward_count: usize,
        backward_count: usize,
        score: PartialScore,
    ) -> ControlFlow<R> {
        // add one to skip the anchor itself when starting to move backward
        let cell = self.anchor.checked_sub(backward_count + 1);

        let cell = match cell {
            None => {
                // adjacent to edge, report and stop
                self.maybe_report(node, forward_count, backward_count, placed, score)?;
                return ControlFlow::Continue(());
            }
            Some(cell) => cell,
        };
        let cell = &self.cells[cell];

        // report if on empty tile
        if cell.letter.is_none() {
            self.maybe_report(node, forward_count, backward_count, placed, score)?;
        }

        // stop if on previous anchor
        if cell.is_anchor() {
            return ControlFlow::Continue(());
        }

        // append next tile
        for trans in node.transitions() {
            let char = trans.inp;
            let next = self.set.as_fst().node(trans.addr);

            if char == b'+' {
                unreachable!()
            } else {
                let letter = Letter::from_char(char as char).unwrap();

                if cell.allowed.get(letter) {
                    let used_deck = if let Some(actual) = cell.letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    let new_score = score.add(used_deck, letter, cell);
                    let new_placed = if used_deck { placed.insert_front(letter) } else { placed };

                    self.run_recurse_backward(deck, next, new_placed, forward_count, backward_count + 1, new_score)?;

                    if used_deck {
                        deck.add(letter);
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[must_use]
    fn maybe_report(
        &mut self,
        node: Node,
        forward_count: usize,
        backward_count: usize,
        placed: Placed,
        score: PartialScore,
    ) -> ControlFlow<R> {
        if !node.is_final() {
            return ControlFlow::Continue(());
        }

        let start = (self.anchor - backward_count) as u8;

        let (x, y) = match self.dir {
            Direction::Horizontal => (start, self.row),
            Direction::Vertical => (self.row, start),
        };

        // TODO rearrange placed based on backward_count?
        let mv = Move {
            dir: self.dir,
            x,
            y,
            forward_count,
            backward_count,
            score: score.total(),
            placed,
        };
        (self.f)(mv)?;

        ControlFlow::Continue(())
    }
}
