use std::fmt::{Display, Formatter};
use std::ops::ControlFlow;

use fst::raw::Node;
use internal_iterator::InternalIterator;
use itertools::Itertools;

use crate::games::scrabble::basic::{Deck, Letter, Mask};
use crate::games::scrabble::movegen;
use crate::games::scrabble::movegen::{movegen, Direction, Move, Set};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Cell {
    // current state
    pub letter: Option<Letter>,

    // intrinsic board state
    pub letter_multiplier: u8,
    pub word_multiplier: u8,

    // prepared information for movegen
    pub allowed_by_dir: [Mask; 2],
    pub score_by_dir: [u32; 2],
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidGridString {
    Empty,
    TooLarge,
    WidthMismatch,
    UnexpectedChar(char),
    NotAscii,
}

const STANDARD_GRID_STR: &str = r#"
=..'...=...'..=
.-..."..."...-.
..-...'.'...-..
'..-...'...-..'
....-.....-....
."..."..."...".
..'...'.'...'..
=..'...-...'..=
..'...'.'...'..
."..."..."...".
....-.....-....
'..-...'...-..'
..-...'.'...-..
.-..."..."...-.
=..'...=...'..=
"#;

impl ScrabbleGrid {
    // only limited by display impl for now
    pub const MAX_SIZE: u8 = 25;

    pub fn standard() -> ScrabbleGrid {
        // if there are no letters yet the set used doesn't matter
        let set = Set::from_iter(std::iter::empty::<String>()).unwrap();
        let s = STANDARD_GRID_STR.trim();
        ScrabbleGrid::from_str_2d(&set, s).unwrap()
    }

    pub fn from_str_2d(set: &Set, s: &str) -> Result<ScrabbleGrid, InvalidGridString> {
        if !s.is_ascii() {
            return Err(InvalidGridString::NotAscii);
        }

        let width = s.lines().next().map_or(0, |s| s.len());
        let height = s.lines().count();

        if width > Self::MAX_SIZE as usize || height > Self::MAX_SIZE as usize {
            return Err(InvalidGridString::TooLarge);
        }
        if width == 0 || height == 0 {
            return Err(InvalidGridString::Empty);
        }

        let mut grid = ScrabbleGrid {
            width: width as u8,
            height: height as u8,
            cells: vec![Cell::empty(); height * width],
        };

        for (y, line) in s.lines().enumerate() {
            let y = y as u8;
            if line.len() != width {
                return Err(InvalidGridString::WidthMismatch);
            }

            for (x, c) in line.chars().enumerate() {
                let x = x as u8;
                match c {
                    // empty square
                    ' ' | '.' => {}

                    // letter
                    'A'..='Z' => grid.set_letter_partial(x, y, Letter::from_char(c).unwrap()),

                    // word multipliers
                    '~' => grid.cell_mut(x, y).word_multiplier = 4,
                    '=' => grid.cell_mut(x, y).word_multiplier = 3,
                    '-' => grid.cell_mut(x, y).word_multiplier = 2,

                    // letter multipliers
                    '^' => grid.cell_mut(x, y).letter_multiplier = 4,
                    '"' => grid.cell_mut(x, y).letter_multiplier = 3,
                    '\'' => grid.cell_mut(x, y).letter_multiplier = 2,

                    _ => return Err(InvalidGridString::UnexpectedChar(c)),
                }
            }
        }

        grid.update_all_allowed(set);
        Ok(grid)
    }

    pub fn copy_multipliers_from(&mut self, other: &ScrabbleGrid) {
        assert!(
            self.width == other.width && self.height == other.height,
            "Shape mismatch: ({}, {}) vs ({}, {})",
            self.width,
            self.height,
            other.width,
            other.height
        );

        for (cell, other_cell) in self.cells.iter_mut().zip(other.cells.iter()) {
            cell.letter_multiplier = other_cell.letter_multiplier;
            cell.word_multiplier = other_cell.word_multiplier;
        }

        // the multipliers don't affect any prepared stuff (not even the scores)
    }

    pub fn cell(&self, x: u8, y: u8) -> &Cell {
        let index = (y as usize) * (self.width as usize) + (x as usize);
        &self.cells[index]
    }

    pub fn cell_mut(&mut self, x: u8, y: u8) -> &mut Cell {
        let index = (y as usize) * (self.width as usize) + (x as usize);
        &mut self.cells[index]
    }

    /// **Warning**: only partial: the neighbor allowed sets and scores are not updated
    pub fn set_letter_partial(&mut self, x: u8, y: u8, letter: Letter) {
        let cell = self.cell_mut(x, y);
        cell.letter = Some(letter);
        cell.allowed_by_dir = [letter.to_mask(), letter.to_mask()];
        cell.attached = false;
        cell.score_by_dir = [0, 0];

        // set neighbors to be attached if empty
        let deltas = [
            (Direction::Horizontal, 1),
            (Direction::Horizontal, -1),
            (Direction::Vertical, 1),
            (Direction::Vertical, -1),
        ];
        for (dir, delta) in deltas {
            if let Some((nx, ny)) = self.neighbor(x, y, dir, delta) {
                let neighbor = self.cell_mut(nx, ny);
                neighbor.attached = neighbor.letter.is_none();
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
                    let (allowed, score) = self.calc_cell_prepared(set, x, y, dir);
                    let cell = self.cell_mut(x, y);
                    cell.allowed_by_dir[dir.index()] = allowed;
                    cell.score_by_dir[dir.index()] = score;
                }
            }
        }
    }

    pub fn assert_valid(&self, set: &Set) {
        let mut clone = self.clone();
        clone.update_all_allowed(set);

        for y in 0..self.height {
            for x in 0..self.width {
                assert_eq!(self.cell(x, y), clone.cell(x, y), "Cell mismatch at ({}, {})", x, y);
            }
        }
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
                    self.set_letter_partial(x, y, c);
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
            self.update_cell_prepared(set, nx, ny, mv.dir);
        }
        if let Some((nx, ny)) = self.neighbor(mv.x, mv.y, mv.dir, mv.word.len() as i16) {
            self.update_cell_prepared(set, nx, ny, mv.dir);
        }

        // orthogonal
        for i in 0..mv.word.len() {
            let (sx, sy) = self.neighbor(mv.x, mv.y, mv.dir, i as i16).unwrap();
            let orthogonal = mv.dir.orthogonal();

            if let Some((nx, ny)) = self.neighbor(sx, sy, orthogonal, -1) {
                self.update_cell_prepared(set, nx, ny, orthogonal)
            }
            if let Some((nx, ny)) = self.neighbor(sx, sy, orthogonal, 1) {
                self.update_cell_prepared(set, nx, ny, orthogonal)
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

    fn calc_cell_prepared(&mut self, set: &Set, x: u8, y: u8, dir: Direction) -> (Mask, u32) {
        // a letter only allows itself
        if let Some(letter) = self.cell(x, y).letter {
            return (letter.to_mask(), 0);
        }

        // otherwise look at the possible completions
        let (prefix, suffix) = self.find_prefix_suffix(x, y, dir);

        // if no adjacent tiles, allow everything
        if prefix.is_empty() && suffix.is_empty() {
            return (Mask::ALL_LETTERS, 0);
        }

        let allowed = find_cross_set(set, &prefix, &suffix);
        debug_assert_eq!(allowed, find_cross_set_slow(set, &prefix, &suffix));

        let score = prefix
            .iter()
            .chain(suffix.iter())
            .map(|&c| Letter::from_char(c as char).unwrap().score_value() as u32)
            .sum();

        (allowed, score)
    }

    fn update_cell_prepared(&mut self, set: &Set, x: u8, y: u8, dir: Direction) {
        let (allowed, score) = self.calc_cell_prepared(set, x, y, dir);

        let cell = self.cell_mut(x, y);
        cell.allowed_by_dir[dir.index()] = allowed;
        cell.score_by_dir[dir.index()] = score;
    }
}

// TODO try getting rid of some unwraps here
fn find_cross_set(set: &Set, prefix: &[u8], suffix: &[u8]) -> Mask {
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

impl Cell {
    fn empty() -> Cell {
        Cell {
            letter: None,
            letter_multiplier: 1,
            word_multiplier: 1,
            allowed_by_dir: [Mask::NONE; 2],
            score_by_dir: [0; 2],
            attached: false,
        }
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
                        attached: cell.attached,
                        allowed: cell.allowed_by_dir[Direction::Vertical.index()],
                        score_cross: cell.score_by_dir[Direction::Vertical.index()],
                        letter_multiplier: cell.letter_multiplier,
                        word_multiplier: cell.word_multiplier,
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
                        attached: cell.attached,
                        allowed: cell.allowed_by_dir[Direction::Horizontal.index()],
                        score_cross: cell.score_by_dir[Direction::Horizontal.index()],
                        letter_multiplier: cell.letter_multiplier,
                        word_multiplier: cell.word_multiplier,
                    }
                })
                .collect_vec();
            movegen(set, Direction::Vertical, x, &cells, deck, &mut f)?;
        }

        ControlFlow::Continue(())
    }
}

impl Display for ScrabbleGrid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "   ")?;
        for x in 0..self.width {
            write!(f, "{} ", (b'A' + x) as char)?;
        }
        writeln!(f)?;

        write!(f, "   ")?;
        for _ in 0..self.width {
            write!(f, "--")?;
        }
        writeln!(f)?;

        for y in 0..self.height {
            write!(f, "{:2}|", y + 1)?;
            for x in 0..self.width {
                let cell = self.cell(x, y);

                let c = match cell.letter {
                    Some(letter) => letter.to_char(),
                    None => match (cell.letter_multiplier, cell.word_multiplier) {
                        (1, 1) => ' ',
                        (1, 2) => '-',
                        (1, 3) => '=',
                        (1, 4) => '~',
                        (2, 1) => '\'',
                        (3, 1) => '"',
                        (4, 1) => '^',
                        _ => '?',
                    },
                };

                write!(f, "{} ", c)?;
            }
            writeln!(f, "|")?;
        }

        write!(f, "   ")?;
        for _ in 0..self.width {
            write!(f, "--")?;
        }
        writeln!(f)?;

        Ok(())
    }
}
