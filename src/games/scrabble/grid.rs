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
    pub allowed_horizontal: Mask,
    pub allowed_vertical: Mask,
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
                        allowed: cell.allowed_vertical,
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
                        allowed: cell.allowed_horizontal,
                        attached: cell.attached,
                    }
                })
                .collect_vec();
            movegen(set, Direction::Vertical, x, &cells, deck, &mut f)?;
        }

        ControlFlow::Continue(())
    }
}
