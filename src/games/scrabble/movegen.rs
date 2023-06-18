use crate::games::scrabble::basic::{Deck, Letter, Mask};
use fst::raw::Node;
use std::ops::ControlFlow;

pub type Set = fst::Set<Vec<u8>>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct Move {
    pub dir: Direction,
    pub row: u8,
    pub anchor: usize,
    pub forward_count: usize,
    pub backward_count: usize,
    pub start: usize,
    pub raw: String,
    pub word: String,
}

#[derive(Debug)]
pub struct Cell {
    pub letter: Option<Letter>,
    pub allowed: Mask,
    pub attached: bool,
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

impl<R, F: FnMut(Move) -> ControlFlow<R>> MoveGen<'_, R, F> {
    fn run(&mut self, deck: Deck) -> ControlFlow<R> {
        let root = self.set.as_fst().root();
        let mut curr = vec![];
        self.run_recurse_forward(deck, root, &mut curr)?;
        debug_assert!(curr.is_empty());

        ControlFlow::Continue(())
    }

    #[must_use]
    fn run_recurse_forward(&mut self, mut deck: Deck, node: Node, curr: &mut Vec<u8>) -> ControlFlow<R> {
        let cell = self.anchor + curr.len();

        // force direction change when hitting the right edge
        if cell >= self.cells.len() {
            if let Some(index) = node.find_input(b'+') {
                let trans = node.transition(index);
                let next = self.set.as_fst().node(trans.addr);
                self.run_recurse_backward(deck, next, curr, curr.len())?;
            }

            return ControlFlow::Continue(());
        }

        for trans in node.transitions() {
            let char = trans.inp;
            let next = self.set.as_fst().node(trans.addr);

            if char == b'+' {
                // switch direction if possible
                if self.cells[cell].letter.is_none() {
                    self.run_recurse_backward(deck, next, curr, curr.len())?;
                }
            } else {
                // place letter if possible
                let letter = Letter::from_char(char as char).unwrap();

                if self.cells[cell].allowed.get(letter) {
                    let used_deck = if let Some(actual) = self.cells[cell].letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    curr.push(char);
                    self.run_recurse_forward(deck, next, curr)?;
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
    ) -> ControlFlow<R> {
        let back_count = curr.len() - forward_count + 1;
        let cell = self.anchor.checked_sub(back_count);

        let cell = match cell {
            None => {
                // adjacent to edge, report and stop
                self.maybe_report(node, forward_count, curr)?;
                return ControlFlow::Continue(());
            }
            Some(cell) => cell,
        };

        // report if on empty tile
        if self.cells[cell].letter.is_none() {
            self.maybe_report(node, forward_count, curr)?;
        }

        // stop if on previous anchor
        if self.cells[cell].is_anchor() {
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

                if self.cells[cell].allowed.get(letter) {
                    let used_deck = if let Some(actual) = self.cells[cell].letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    curr.push(char);
                    self.run_recurse_backward(deck, next, curr, forward_count)?;
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
    fn maybe_report(&mut self, node: Node, forward_count: usize, word: &[u8]) -> ControlFlow<R> {
        if node.is_final() {
            let backward_count = word.len() - forward_count;

            let prefix = &word[..forward_count];
            let suffix_rev = &word[forward_count..];

            let mut ordered = vec![];
            ordered.extend(suffix_rev.iter().copied().rev());
            ordered.extend_from_slice(prefix);

            let mv = Move {
                dir: self.dir,
                row: self.row,
                backward_count,
                forward_count,
                anchor: self.anchor,
                start: self.anchor - backward_count,
                raw: String::from_utf8(word.to_owned()).unwrap(),
                word: String::from_utf8(ordered).unwrap(),
            };
            (self.f)(mv)?;
        }

        ControlFlow::Continue(())
    }
}
