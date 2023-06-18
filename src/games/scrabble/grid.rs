use std::ops::ControlFlow;

use fst::raw::Node;
use internal_iterator::InternalIterator;
use itertools::Itertools;

use crate::games::scrabble::basic::{Deck, Letter, Mask};
use crate::games::scrabble::movegen;
use crate::games::scrabble::movegen::{movegen, Direction, Move, Set};

#[derive(Debug, Clone)]
pub struct Cell {
    pub letter: Option<Letter>,
    pub letter_multiplier: u8,
    pub word_multiplier: u8,
    pub allowed_by_horizontal: Mask,
    pub allowed_by_vertical: Mask,
    pub attached: bool,
}

#[derive(Debug, Clone)]
pub struct ScrabbleGrid {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<Cell>,
}

impl ScrabbleGrid {
    pub fn cell(&self, x: u8, y: u8) -> &Cell {
        let index = (y as usize) * (self.width as usize) + (x as usize);
        &self.cells[index]
    }

    pub fn cell_mut(&mut self, x: u8, y: u8) -> &mut Cell {
        let index = (y as usize) * (self.width as usize) + (x as usize);
        &mut self.cells[index]
    }

    pub fn available_moves<'s>(&self, set: &'s Set, deck: Deck) -> MovesIterator<'_, 's> {
        MovesIterator { grid: self, set, deck }
    }

    pub fn recompute_allowed(&mut self, set: &Set) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.cell_mut(x, y).allowed_by_horizontal = self.calc_cell_allowed(set, x, y, Direction::Horizontal);
                self.cell_mut(x, y).allowed_by_vertical = self.calc_cell_allowed(set, x, y, Direction::Vertical);
            }
        }
    }

    fn find_prefix_suffix(&mut self, x: u8, y: u8, dir: Direction) -> (Vec<u8>, Vec<u8>) {
        let (dx, dy) = dir.delta();

        // prefix
        let mut prefix = vec![];

        let mut px = x;
        let mut py = y;
        while px >= dx && py >= dy && self.cell(px - dx, py - dy).letter.is_some() {
            px -= dx;
            py -= dy;
        }
        for cx in px..x {
            prefix.push(self.cell(cx, y).letter.unwrap().to_ascii());
        }
        for cy in py..y {
            prefix.push(self.cell(x, cy).letter.unwrap().to_ascii());
        }

        // suffix
        let mut suffix = vec![];
        let mut sx = x + dx;
        let mut sy = y + dy;
        while sx < self.width && sy < self.height {
            match self.cell(sx, sy).letter {
                None => break,
                Some(letter) => suffix.push(letter.to_ascii()),
            }
            sx += dx;
            sy += dy;
        }

        (prefix, suffix)
    }

    fn calc_cell_allowed(&mut self, set: &Set, x: u8, y: u8, dir: Direction) -> Mask {
        // a letter only allows itself
        if let Some(letter) = self.cell(x, y).letter {
            return letter.to_mask();
        }

        // otherwise look at the possible completions
        let (prefix, suffix) = self.find_prefix_suffix(x, y, dir);

        let mask = find_cross_set(set, &prefix, &suffix);
        debug_assert_eq!(mask, find_cross_set_slow(set, &prefix, &suffix));

        mask
    }
}

fn find_cross_set(set: &Set, prefix: &[u8], suffix: &[u8]) -> Mask {
    let fst = set.as_fst();

    match (prefix, suffix) {
        (&[], &[]) => Mask::ALL_LETTERS,
        (prefix, &[]) => {
            // TODO it would be easier to leave off the final + in the set here
            // look for chars 'c' with path
            // root --"prefix"-> node_start --'c'-> node_mid --'+'-> node_final

            let node_start = fst_follow(set, fst.root(), prefix).expect("invalid word on the board");

            let mut mask = Mask::NONE;

            for trans in node_start.transitions() {
                let node_mid = fst.node(trans.addr);

                if let Some(node_final) = fst_follow(set, node_mid, &[b'+']) {
                    let letter = Letter::from_char(trans.inp as char).unwrap();
                    mask.set(letter, node_final.is_final());
                }
            }

            mask
        }
        (&[], suffix) => {
            // look for chars 'c' with path
            // root --"suffix"-> node_start --'+'-> node_mid --'c'-> node_final

            let node_start = fst_follow(set, fst.root(), suffix).expect("invalid word on the board");
            let node_mid = fst_follow(set, node_start, &[b'+']).expect("invalid word on the board");

            let mut mask = Mask::NONE;

            for trans in node_mid.transitions() {
                let letter = Letter::from_char(trans.inp as char).unwrap();

                let node_final = fst.node(trans.addr);
                mask.set(letter, node_final.is_final());
            }

            mask
        }
        (prefix, suffix) => {
            // look for chars 'c' with path
            // root --"suffix"-> node_start --'+'-> node_mid --'c'-> node_next --rev("prefix")--> node_end

            let node_start = fst_follow(set, fst.root(), suffix).expect("invalid word on the board");
            let node_mid = fst_follow(set, node_start, &[b'+']).expect("invalid word on the board");

            let mut mask = Mask::NONE;

            for trans in node_mid.transitions() {
                let letter = Letter::from_char(trans.inp as char).unwrap();

                let node_next = fst.node(trans.addr);
                if let Some(node_end) = fst_follow(set, node_next, prefix.iter().rev()) {
                    mask.set(letter, node_end.is_final());
                }
            }

            mask
        }
    }
}

fn fst_follow<'s, 'a>(set: &'s Set, start: Node<'s>, sequence: impl IntoIterator<Item = &'a u8>) -> Option<Node<'s>> {
    let fst = set.as_fst();

    let mut node = start;
    for &v in sequence {
        let index = node.find_input(v)?;
        node = fst.node(node.transition_addr(index));
    }

    Some(node)
}

fn find_cross_set_slow(set: &Set, prefix: &[u8], suffix: &[u8]) -> Mask {
    if prefix.is_empty() && suffix.is_empty() {
        return Mask::ALL_LETTERS;
    }

    let mut mask = Mask::NONE;
    let mut word = vec![];

    for c in Letter::all() {
        word.clear();
        word.extend_from_slice(prefix);
        word.push(c.to_ascii());
        word.extend_from_slice(suffix);
        word.push(b'+');

        mask.set(c, set.contains(&word));
    }

    mask
}

#[derive(Debug, Clone)]
pub struct MovesIterator<'g, 's> {
    grid: &'g ScrabbleGrid,
    set: &'s Set,
    deck: Deck,
}

impl InternalIterator for MovesIterator<'_, '_> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let MovesIterator { grid, set, deck } = self;

        // horizontal
        for y in 0..grid.height {
            let cells = (0..grid.width)
                .map(|x| {
                    let cell = grid.cell(x, y);
                    movegen::Cell {
                        letter: cell.letter,
                        allowed: cell.allowed_by_vertical,
                        attached: cell.attached,
                    }
                })
                .collect_vec();

            movegen(set, Direction::Horizontal, y, &cells, deck, &mut f)?;
        }

        // vertical
        for x in 0..grid.width {
            let cells = (0..grid.height)
                .map(|y| {
                    let cell = grid.cell(x, y);
                    movegen::Cell {
                        letter: cell.letter,
                        allowed: cell.allowed_by_horizontal,
                        attached: cell.attached,
                    }
                })
                .collect_vec();
            movegen(set, Direction::Vertical, x, &cells, deck, &mut f)?;
        }

        ControlFlow::Continue(())
    }
}
