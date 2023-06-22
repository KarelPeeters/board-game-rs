use std::fmt::{Debug, Formatter};
use std::ops::ControlFlow;

use fst::raw::Node;

use crate::games::scrabble::basic::{Deck, Letter, Mask, MAX_DECK_SIZE};
use crate::games::scrabble::grid::ScrabbleGrid;

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
pub struct PlaceMove {
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

    pub forward_count: u8,
    pub backward_count: u8,

    pub score: u32,
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

struct MoveGen<'a, R, F: FnMut(PlaceMove) -> ControlFlow<R>, const VERTICAL: bool> {
    set: &'a Set,
    grid: &'a ScrabbleGrid,

    orthogonal_index: u8,
    anchor: u8,
    len: u8,

    f: F,
}

#[must_use]
pub fn movegen<R, const VERTICAL: bool>(
    set: &Set,
    grid: &ScrabbleGrid,
    orthogonal_index: u8,
    deck: Deck,
    mut f: impl FnMut(PlaceMove) -> ControlFlow<R>,
) -> ControlFlow<R> {
    assert!(deck.count() as usize <= MAX_DECK_SIZE);

    let len = match VERTICAL {
        false => grid.width,
        true => grid.height,
    };

    for anchor in 0..len {
        let mut movegen = MoveGen::<_, _, VERTICAL> {
            grid,
            set,
            orthogonal_index,
            len,
            anchor,
            f: &mut f,
        };
        if !movegen.cell(anchor).anchor {
            continue;
        }

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

    #[allow(clippy::len_without_is_empty)]
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
    fn add(mut self, placed: bool, letter: Letter, cell: &CellInfo) -> Self {
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
            let score_cross = cell.score_orthogonal;
            if score_cross != 0 {
                self.sum_orthogonal += (letter_value + score_cross) * word_mul;
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

#[derive(Debug, Copy, Clone)]
struct CellInfo {
    anchor: bool,
    letter: Option<Letter>,
    allowed: Mask,
    score_orthogonal: u32,
    letter_multiplier: u8,
    word_multiplier: u8,
}

impl<R, F: FnMut(PlaceMove) -> ControlFlow<R>, const VERTICAL: bool> MoveGen<'_, R, F, VERTICAL> {
    fn map_to_xy(&self, cell: u8) -> (u8, u8) {
        match VERTICAL {
            false => (cell, self.orthogonal_index),
            true => (self.orthogonal_index, cell),
        }
    }

    fn cell(&self, cell: u8) -> CellInfo {
        let (x, y) = self.map_to_xy(cell);
        let cell = self.grid.cell(x, y);
        let anchor = self.grid.attached(x, y);

        let orthogonal_index = (!VERTICAL) as usize;

        CellInfo {
            anchor,
            letter: cell.letter,
            allowed: cell.cache_allowed_by_dir[orthogonal_index],
            score_orthogonal: cell.cache_score_by_dir[orthogonal_index],
            letter_multiplier: cell.letter_multiplier,
            word_multiplier: cell.word_multiplier,
        }
    }

    fn run(&mut self, deck: Deck) -> ControlFlow<R> {
        let root = self.set.as_fst().root();
        self.run_recurse_forward(deck, root, 0, Placed::default(), PartialScore::default())
    }

    #[must_use]
    fn run_recurse_forward(
        &mut self,
        deck: Deck,
        node: Node,
        forward_count: u8,
        placed: Placed,
        score: PartialScore,
    ) -> ControlFlow<R> {
        let fst = self.set.as_fst();
        let cell = self.anchor + forward_count;

        // force direction change when hitting the right edge
        if cell >= self.len {
            if let Some(index) = node.find_input(b'+') {
                let trans_addr = node.transition_addr(index);
                let next = fst.node(trans_addr);
                self.run_recurse_backward(deck, next, placed, forward_count, 0, score)?;
            }

            return ControlFlow::Continue(());
        }
        let cell = self.cell(cell);

        if let Some(letter) = cell.letter {
            // forcibly follow the given letter
            if let Some(next_index) = node.find_input(letter.to_ascii()) {
                let next = fst.node(node.transition_addr(next_index));

                let new_score = score.add(false, letter, &cell);
                self.run_recurse_forward(deck, next, forward_count + 1, placed, new_score)?;
            }
        } else {
            // empty space, we can switch direction
            if let Some(next) = node.find_input(b'+') {
                let next = fst.node(node.transition_addr(next));

                self.run_recurse_backward(deck, next, placed, forward_count, 0, score)?;
            }

            // handle placing new letter
            let allowed = cell.allowed & deck.usable_mask();

            if (allowed.count() as usize) < node.len() {
                for letter in allowed.letters() {
                    if let Some(next_index) = node.find_input(letter.to_ascii()) {
                        let next = fst.node(node.transition_addr(next_index));

                        let mut new_deck = deck;
                        new_deck.remove(letter);
                        let new_placed = placed.push_back(letter);
                        let new_score = score.add(true, letter, &cell);

                        self.run_recurse_forward(new_deck, next, forward_count + 1, new_placed, new_score)?;
                    }
                }
            } else {
                for trans in node.transitions() {
                    let char = trans.inp;
                    if char == b'+' {
                        continue;
                    }
                    let letter = Letter::from_char(char as char).unwrap();
                    if !allowed.get(letter) {
                        continue;
                    }

                    let next = fst.node(trans.addr);

                    let mut new_deck = deck;
                    new_deck.remove(letter);
                    let new_placed = placed.push_back(letter);
                    let new_score = score.add(true, letter, &cell);

                    self.run_recurse_forward(new_deck, next, forward_count + 1, new_placed, new_score)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[must_use]
    fn run_recurse_backward(
        &mut self,
        deck: Deck,
        node: Node,
        placed: Placed,
        forward_count: u8,
        backward_count: u8,
        score: PartialScore,
    ) -> ControlFlow<R> {
        let fst = self.set.as_fst();

        // add one to skip the anchor itself when starting to move backward
        let cell = self.anchor.checked_sub(backward_count + 1);
        let cell = match cell {
            None => {
                // hit edge, report and stop
                self.maybe_report(node, forward_count, backward_count, placed, score)?;
                return ControlFlow::Continue(());
            }
            Some(cell) => cell,
        };
        let cell = self.cell(cell);

        // report if on empty tile
        if cell.letter.is_none() {
            self.maybe_report(node, forward_count, backward_count, placed, score)?;
        }

        // stop if on previous anchor
        if cell.anchor {
            return ControlFlow::Continue(());
        }

        if let Some(letter) = cell.letter {
            // forcibly follow the given letter
            if let Some(next_index) = node.find_input(letter.to_ascii()) {
                let next = fst.node(node.transition_addr(next_index));

                let new_score = score.add(false, letter, &cell);
                self.run_recurse_backward(deck, next, placed, forward_count, backward_count + 1, new_score)?;
            }
        } else {
            // handle placing new letter
            let allowed = cell.allowed & deck.usable_mask();

            if (allowed.count() as usize) < node.len() {
                for letter in allowed.letters() {
                    if let Some(next_index) = node.find_input(letter.to_ascii()) {
                        let next = fst.node(node.transition_addr(next_index));

                        let mut new_deck = deck;
                        new_deck.remove(letter);
                        let new_placed = placed.insert_front(letter);
                        let new_score = score.add(true, letter, &cell);

                        self.run_recurse_backward(
                            new_deck,
                            next,
                            new_placed,
                            forward_count,
                            backward_count + 1,
                            new_score,
                        )?;
                    }
                }
            } else {
                for trans in node.transitions() {
                    let char = trans.inp;
                    let letter = Letter::from_char(char as char).unwrap();
                    if !allowed.get(letter) {
                        continue;
                    }

                    let next = fst.node(trans.addr);

                    let mut new_deck = deck;
                    new_deck.remove(letter);
                    let new_placed = placed.insert_front(letter);
                    let new_score = score.add(true, letter, &cell);

                    self.run_recurse_backward(
                        new_deck,
                        next,
                        new_placed,
                        forward_count,
                        backward_count + 1,
                        new_score,
                    )?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[must_use]
    fn maybe_report(
        &mut self,
        node: Node,
        forward_count: u8,
        backward_count: u8,
        placed: Placed,
        score: PartialScore,
    ) -> ControlFlow<R> {
        if !node.is_final() {
            return ControlFlow::Continue(());
        }

        let start = self.anchor - backward_count;
        let (x, y) = self.map_to_xy(start);
        let dir = match VERTICAL {
            false => Direction::Horizontal,
            true => Direction::Vertical,
        };

        let mv = PlaceMove {
            dir,
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
