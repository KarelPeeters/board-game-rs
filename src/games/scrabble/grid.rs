use std::ops::ControlFlow;

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
        let set = find_cross_set(set, &prefix, &suffix);

        if !(prefix.is_empty() && suffix.is_empty()) {
            println!(
                "cell ({}, {}) dir {:?} => prefix {:?} suffix {:?} => {:?}",
                x,
                y,
                dir,
                std::str::from_utf8(&prefix).unwrap(),
                std::str::from_utf8(&suffix).unwrap(),
                set,
            );
        }

        set
    }
}

fn find_cross_set(set: &Set, prefix: &[u8], suffix: &[u8]) -> Mask {
    find_cross_set_slow(set, prefix, suffix)

    // match (prefix.is_empty(), suffix.is_empty()) {
    //     (true, true) => Mask::ALL_LETTERS,
    //     (true, false) => {}
    //     (false, true) => todo!(),
    //     (false, false) => todo!(),
    // }
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
