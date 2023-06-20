use std::ops::ControlFlow;

use fst::raw::Node;

use crate::games::scrabble::basic::{Deck, Letter, Mask};

pub type Set = fst::Set<Vec<u8>>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct Move {
    // TODO encode which tiles use a wildcard tile
    //   does it ever make sense to use a wildcard when not necessary?
    //   does it make sense to place the wildcard on a multiplier tile when possible to avoid?
    pub dir: Direction,
    pub x: u8,
    pub y: u8,

    pub score: u32,

    pub anchor: u8,
    pub forward_count: u8,
    pub backward_count: u8,
    pub raw: String,
    pub word: String,
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
        let mut curr = vec![];
        self.run_recurse_forward(deck, root, &mut curr, PartialScore::default())?;
        debug_assert!(curr.is_empty());

        ControlFlow::Continue(())
    }

    #[must_use]
    fn run_recurse_forward(
        &mut self,
        mut deck: Deck,
        node: Node,
        curr: &mut Vec<u8>,
        score: PartialScore,
    ) -> ControlFlow<R> {
        let cell = self.anchor + curr.len();

        // force direction change when hitting the right edge
        if cell >= self.cells.len() {
            if let Some(index) = node.find_input(b'+') {
                let trans = node.transition(index);
                let next = self.set.as_fst().node(trans.addr);
                self.run_recurse_backward(deck, next, curr, curr.len(), score)?;
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
                    self.run_recurse_backward(deck, next, curr, curr.len(), score)?;
                }
            } else {
                // place letter if possible
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

                    curr.push(char);
                    let new_score = score.add(used_deck, letter, cell);

                    self.run_recurse_forward(deck, next, curr, new_score)?;
                    let popped = curr.pop();
                    debug_assert_eq!(popped, Some(char));

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
        curr: &mut Vec<u8>,
        forward_count: usize,
        score: PartialScore,
    ) -> ControlFlow<R> {
        debug_assert!(forward_count > 0);

        let back_count = curr.len() - forward_count + 1;
        let cell = self.anchor.checked_sub(back_count);

        let cell = match cell {
            None => {
                // adjacent to edge, report and stop
                self.maybe_report(node, forward_count, curr, score)?;
                return ControlFlow::Continue(());
            }
            Some(cell) => cell,
        };
        let cell = &self.cells[cell];

        // report if on empty tile
        if cell.letter.is_none() {
            self.maybe_report(node, forward_count, curr, score)?;
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

                    curr.push(char);
                    let new_score = score.add(used_deck, letter, cell);

                    self.run_recurse_backward(deck, next, curr, forward_count, new_score)?;
                    curr.pop();

                    if used_deck {
                        deck.add(letter);
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[must_use]
    fn maybe_report(&mut self, node: Node, forward_count: usize, word: &[u8], score: PartialScore) -> ControlFlow<R> {
        if node.is_final() {
            let backward_count = word.len() - forward_count;

            let prefix = &word[..forward_count];
            let suffix_rev = &word[forward_count..];

            let mut ordered = vec![];
            ordered.extend(suffix_rev.iter().copied().rev());
            ordered.extend_from_slice(prefix);

            let start = (self.anchor - backward_count) as u8;

            let (x, y) = match self.dir {
                Direction::Horizontal => (start, self.row),
                Direction::Vertical => (self.row, start),
            };

            let mv = Move {
                dir: self.dir,
                x,
                y,
                score: score.total(),
                backward_count: backward_count as u8,
                forward_count: forward_count as u8,
                anchor: self.anchor as u8,
                raw: String::from_utf8(word.to_owned()).unwrap(),
                word: String::from_utf8(ordered).unwrap(),
            };
            (self.f)(mv)?;
        }

        ControlFlow::Continue(())
    }
}
