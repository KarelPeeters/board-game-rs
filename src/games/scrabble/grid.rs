use std::ops::ControlFlow;

use fst::raw::Node;
use internal_iterator::InternalIterator;
use itertools::Itertools;

use crate::games::scrabble::basic::{Deck, Letter, Mask};
use crate::games::scrabble::movegen;
use crate::games::scrabble::movegen::{movegen, Direction, Move, Set};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Cell {
    pub letter: Option<Letter>,
    pub letter_multiplier: u8,
    pub word_multiplier: u8,
    pub allowed_by_dir: [Mask; 2],
    pub attached: bool,
}

#[derive(Debug, Clone)]
pub struct ScrabbleGrid {
    pub width: u8,
    pub height: u8,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidMove {
    OutOfBounds,
    ConflictExisting,
    NotInDeck(Letter),

    NotAttached,
    NoNewTiles,
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

    pub fn set_letter_without_allowed(&mut self, x: u8, y: u8, letter: Letter) {
        let cell = self.cell_mut(x, y);
        cell.letter = Some(letter);
        cell.allowed_by_dir = [letter.to_mask(), letter.to_mask()];
        cell.attached = false;

        // set neighbors to be attached if empty
        let neighbors = [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1)),
        ];
        for (nx, ny) in neighbors {
            if let (Some(nx), Some(ny)) = (nx, ny) {
                if nx < self.width && ny < self.height {
                    let cell = self.cell_mut(x, y);
                    cell.attached = cell.letter.is_none();
                }
            }
        }
    }

    pub fn available_moves<'s>(&self, set: &'s Set, deck: Deck) -> MovesIterator<'_, 's> {
        MovesIterator { grid: self, set, deck }
    }

    pub fn update_all_allowed(&mut self, set: &Set) {
        for y in 0..self.height {
            for x in 0..self.width {
                for dir in Direction::ALL {
                    self.cell_mut(x, y).allowed_by_dir[dir.index()] = self.calc_cell_allowed(set, x, y, dir);
                }
            }
        }
    }

    pub fn assert_valid(&self, set: &Set) {
        let mut clone = self.clone();
        clone.update_all_allowed(set);
        assert_eq!(self.cells, clone.cells);
    }

    fn neighbor(&self, x: u8, y: u8, dir: Direction, delta: i16) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();

        // for the some reason IDE function resolution fails here, so we have to help a bit
        let checked_add = core::primitive::i16::checked_add;
        let checked_mul = core::primitive::i16::checked_mul;

        let nx = checked_add(x as i16, checked_mul(delta, dx as i16)?)?;
        let ny = checked_add(y as i16, checked_mul(delta, dy as i16)?)?;

        if 0 <= nx && nx < self.width as i16 && 0 <= ny && ny < self.height as i16 {
            Some((nx as u8, ny as u8))
        } else {
            None
        }
    }

    pub fn play(&mut self, set: &Set, mv: Move, mut deck: Deck) -> Result<Deck, InvalidMove> {
        let mut attached = false;
        let mut placed = false;

        // place new letters
        for (i, c) in mv.word.bytes().enumerate() {
            let (x, y) = self
                .neighbor(mv.x, mv.y, mv.dir, i as i16)
                .ok_or(InvalidMove::OutOfBounds)?;
            let c = Letter::from_char(c as char).unwrap();

            let cell = self.cell_mut(x, y);
            attached |= cell.attached;

            match cell.letter {
                None => {
                    placed = true;
                    if !deck.try_remove(c) {
                        return Err(InvalidMove::NotInDeck(c));
                    }
                    self.set_letter_without_allowed(x, y, c);
                }
                Some(prev) => {
                    if prev != c {
                        return Err(InvalidMove::ConflictExisting);
                    }
                }
            }
        }

        // check that we placed something and that it was attached
        if !attached {
            return Err(InvalidMove::NotAttached);
        }
        if !placed {
            return Err(InvalidMove::NoNewTiles);
        }

        // update validness
        // TODO we can could the node between the prefix and suffix search

        // prefix and suffix
        if let Some((nx, ny)) = self.neighbor(mv.x, mv.y, mv.dir, -1) {
            self.cell_mut(nx, ny).allowed_by_dir[mv.dir.index()] = self.calc_cell_allowed(set, nx, ny, mv.dir)
        }
        if let Some((nx, ny)) = self.neighbor(mv.x, mv.y, mv.dir, mv.word.len() as i16) {
            self.cell_mut(nx, ny).allowed_by_dir[mv.dir.index()] = self.calc_cell_allowed(set, nx, ny, mv.dir)
        }

        // orthogonal
        for i in 0..mv.word.len() {
            let (sx, sy) = self.neighbor(mv.x, mv.y, mv.dir, i as i16).unwrap();
            let orthogonal = mv.dir.orthogonal();

            if let Some((nx, ny)) = self.neighbor(sx, sy, orthogonal, -1) {
                self.cell_mut(nx, ny).allowed_by_dir[orthogonal.index()] =
                    self.calc_cell_allowed(set, nx, ny, orthogonal)
            }
            if let Some((nx, ny)) = self.neighbor(sx, sy, orthogonal, 1) {
                self.cell_mut(nx, ny).allowed_by_dir[orthogonal.index()] =
                    self.calc_cell_allowed(set, nx, ny, orthogonal)
            }
        }

        Ok(deck)
    }

    fn find_prefix_suffix(&mut self, x: u8, y: u8, dir: Direction) -> (Vec<u8>, Vec<u8>) {
        // TODO rewrite using neighbor?
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

// TODO try getting rid of some unwraps here
fn find_cross_set(set: &Set, prefix: &[u8], suffix: &[u8]) -> Mask {
    if prefix.is_empty() && suffix.is_empty() {
        return Mask::ALL_LETTERS;
    }

    let fst = set.as_fst();
    let mut mask = Mask::NONE;

    // pick the order with most certain transitions (including the fixed '+')
    if prefix.len() > suffix.len() {
        // look for chars 'c' with path
        // [root] -> prefix -> 'c' -> suffix -> '+' -> [final]

        let node_prefix = fst_follow(set, fst.root(), prefix).expect("invalid word on the board");

        for trans in node_prefix.transitions() {
            if trans.inp == b'+' {
                continue;
            }

            let c = Letter::from_char(trans.inp as char).unwrap();
            let node_c = fst.node(trans.addr);

            if let Some(node_suffix) = fst_follow(set, node_c, suffix) {
                if let Some(node_plus) = fst_follow(set, node_suffix, &[b'+']) {
                    mask.set(c, node_plus.is_final());
                }
            }
        }
    } else {
        // look for chars 'c' with path
        // [root] -> suffix -> '+' -> 'c' -> rev(prefix) -> [final]

        let node_suffix = fst_follow(set, fst.root(), suffix).expect("invalid word on the board");
        let node_plus = fst_follow(set, node_suffix, &[b'+']).expect("invalid word on the board");

        for trans in node_plus.transitions() {
            let c = Letter::from_char(trans.inp as char).unwrap();
            let node_c = fst.node(trans.addr);

            if let Some(node_prefix) = fst_follow(set, node_c, prefix.iter().rev()) {
                mask.set(c, node_prefix.is_final());
            }
        }
    }

    mask
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
                        allowed: cell.allowed_by_dir[Direction::Vertical.index()],
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
                        allowed: cell.allowed_by_dir[Direction::Horizontal.index()],
                        attached: cell.attached,
                    }
                })
                .collect_vec();
            movegen(set, Direction::Vertical, x, &cells, deck, &mut f)?;
        }

        ControlFlow::Continue(())
    }
}
