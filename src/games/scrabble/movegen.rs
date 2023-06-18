use crate::games::scrabble::basic::{Deck, Letter, Mask};
use fst::raw::Node;
use std::ops::ControlFlow;

pub type Set = fst::Set<Vec<u8>>;

#[derive(Debug)]
pub struct Move {
    pub anchor: usize,
    pub forward_count: usize,
    pub word: String,
}

pub struct Cell {
    pub letter: Option<Letter>,
    pub allowed: Mask,
    pub has_neighbor: bool,
}

struct MoveGen<'a, R, F: FnMut(Move) -> ControlFlow<R>> {
    set: &'a Set,
    cells: &'a [Cell],
    anchor: usize,
    max_back: usize,
    f: F,
}

#[must_use]
pub fn movegen<R>(set: &Set, cells: &[Cell], deck: Deck, mut f: impl FnMut(Move) -> ControlFlow<R>) -> ControlFlow<R> {
    let mut prev_anchor = None;

    for curr in 0..cells.len() {
        let cell = &cells[curr];
        let is_anchor = cell.letter.is_none() && cell.has_neighbor;
        if !is_anchor {
            continue;
        }

        let max_back = prev_anchor.map_or(curr, |prev| curr - prev - 1);
        prev_anchor = Some(curr);

        let mut movegen = MoveGen {
            set,
            cells,
            anchor: curr,
            max_back,
            f: &mut f,
        };
        movegen.run(deck)?;
    }

    ControlFlow::Continue(())
}

impl<R, F: FnMut(Move) -> ControlFlow<R>> MoveGen<'_, R, F> {
    fn run(&mut self, deck: Deck) -> ControlFlow<R> {
        println!(
            "starting movegen anchor={} max_back={} {:?}",
            self.anchor, self.max_back, deck
        );

        let root = self.set.as_fst().root();
        let mut curr = vec![];
        self.run_recurse_forward(deck, root, &mut curr)?;
        debug_assert!(curr.is_empty());

        ControlFlow::Continue(())
    }

    #[must_use]
    fn run_recurse_forward(&mut self, mut deck: Deck, node: Node, curr: &mut Vec<u8>) -> ControlFlow<R> {
        println!("run_recurse_forward {:?} {:?}", deck, curr);

        self.maybe_report(node, curr.len(), curr)?;

        let cell = self.anchor + curr.len();
        if cell >= self.cells.len() {
            // hit right edge, we have to switch direction now
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
                // switch direction
                self.run_recurse_backward(deck, next, curr, curr.len())?;
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
        println!("run_recurse_back {:?} {:?} {}", deck, curr, forward_count);

        self.maybe_report(node, forward_count, curr)?;

        let back_count = curr.len() - forward_count;
        if back_count >= self.max_back {
            // we hit the left edge, stop
            return ControlFlow::Continue(());
        }
        let cell = self.anchor - back_count;

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
            let mv = Move {
                anchor: self.anchor,
                forward_count,
                word: String::from_utf8(word.to_vec()).unwrap(),
            };
            (self.f)(mv)?;
        }

        ControlFlow::Continue(())
    }
}
