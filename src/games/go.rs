use std::cmp::Ordering;
use std::fmt::Write;
use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;

use internal_iterator::InternalIterator;
use itertools::Itertools;

use crate::board::{
    AllMovesIterator, AvailableMovesIterator, Board, BoardDone, BoardMoves, Outcome, PlayError, Player,
};
use crate::impl_unit_symmetry_board;

/// The specific Go rules used.
/// See [KataGo's supported rules](https://lightvector.github.io/KataGo/rules.html) for an overview of the variants.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Rules {
    allow_suicide: bool,
}

impl Rules {
    /// Tromp-Taylor rules, see https://tromp.github.io/go.html.
    pub fn tromp_taylor() -> Self {
        Rules { allow_suicide: true }
    }

    /// Rules used by the [Computer Go Server](http://www.yss-aya.com/cgos/).
    /// The same as Tromp-Taylor except suicide is not allowed.
    pub fn cgos() -> Self {
        Rules { allow_suicide: false }
    }
}

// TODO write better debug impl
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct GoBoard {
    size: u8,
    rules: Rules,

    tiles: Vec<Option<Player>>,
    next_player: Player,
    state: State,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Move {
    Pass,
    Place(Tile),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Tile {
    pub x: u8,
    pub y: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [Direction::Up, Direction::Down, Direction::Left, Direction::Right];
}

impl Tile {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub fn all(size: u8) -> impl Iterator<Item = Tile> {
        (0..size).flat_map(move |y| (0..size).map(move |x| Tile::new(x, y)))
    }

    pub fn index(&self, size: u8) -> usize {
        self.y as usize * size as usize + self.x as usize
    }

    pub fn adjacent(&self, dir: Direction, size: u8) -> Option<Tile> {
        let (x, y) = match dir {
            Direction::Up => (self.x, self.y.checked_add(1)?),
            Direction::Down => (self.x, self.y.checked_sub(1)?),
            Direction::Left => (self.x.checked_sub(1)?, self.y),
            Direction::Right => (self.x.checked_add(1)?, self.y),
        };
        if x < size && y < size {
            Some(Tile::new(x, y))
        } else {
            None
        }
    }

    pub fn exists(&self, size: u8) -> bool {
        self.x < size && self.y < size
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Score {
    a: u32,
    b: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum State {
    Normal,
    Passed,
    Done(Outcome),
}

impl GoBoard {
    pub const MAX_SIZE: u8 = 19;

    pub fn new(size: u8, rules: Rules) -> GoBoard {
        assert!(size <= Self::MAX_SIZE);
        assert_eq!(rules, Rules::tromp_taylor());

        GoBoard {
            size,
            rules,
            tiles: vec![None; size as usize * size as usize],
            next_player: Player::A,
            state: State::Normal,
        }
    }

    pub fn tile(&self, tile: Tile) -> Option<Player> {
        self.tiles[tile.index(self.size)]
    }

    fn tile_mut(&mut self, tile: Tile) -> &mut Option<Player> {
        &mut self.tiles[tile.index(self.size)]
    }

    pub fn size(&self) -> u8 {
        self.size
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn current_score(&self) -> Score {
        let mut score_a = 0;
        let mut score_b = 0;

        for tile in Tile::all(self.size) {
            match self.tile(tile) {
                None => {
                    let reaches_a = self.reaches(tile, Some(Player::A));
                    let reaches_b = self.reaches(tile, Some(Player::B));
                    match (reaches_a, reaches_b) {
                        (true, false) => score_a += 1,
                        (false, true) => score_b += 1,
                        (true, true) | (false, false) => {}
                    }
                }
                Some(Player::A) => score_a += 1,
                Some(Player::B) => score_b += 1,
            }
        }
        Score { a: score_a, b: score_b }
    }

    /// Is there a path between `start` and another tile with value `target` over only `player` tiles?
    fn reaches(&self, start: Tile, target: Option<Player>) -> bool {
        let through = self.tile(start);
        assert_ne!(through, target);

        let mut visited = vec![false; self.tiles.len()];
        let mut stack = vec![start];

        while let Some(tile) = stack.pop() {
            let index = tile.index(self.size);
            if visited[index] {
                continue;
            }
            visited[index] = true;

            for dir in Direction::ALL {
                if let Some(adj) = tile.adjacent(dir, self.size) {
                    let value = self.tile(adj);
                    if value == target {
                        return true;
                    }
                    if value == through {
                        stack.push(adj);
                    }
                }
            }
        }

        false
    }

    fn clear(&mut self, player: Player) -> bool {
        let mut to_clear = vec![];

        for tile in Tile::all(self.size) {
            if self.tile(tile) == Some(player) && !self.reaches(tile, None) {
                to_clear.push(tile);
            }
        }

        for &tile in &to_clear {
            *self.tile_mut(tile) = None;
        }

        !to_clear.is_empty()
    }

    fn update_state_passed(&mut self) {
        self.state = match self.state {
            State::Normal => State::Passed,
            State::Passed => {
                let score = self.current_score();
                let outcome = match score.a.cmp(&score.b) {
                    Ordering::Greater => Outcome::WonBy(Player::A),
                    Ordering::Equal => Outcome::Draw,
                    Ordering::Less => Outcome::WonBy(Player::B),
                };
                State::Done(outcome)
            }
            State::Done(_) => unreachable!(),
        };
    }
}

impl Board for GoBoard {
    type Move = Move;

    fn next_player(&self) -> Player {
        self.next_player
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;

        match mv {
            Move::Pass => Ok(true),
            // TODO ensure the board would not repeat by playing at `tile`
            Move::Place(tile) => {
                if !tile.exists(self.size) {
                    Ok(false)
                } else {
                    Ok(self.tile(tile).is_none())
                }
            }
        }
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_can_play(mv)?;

        match mv {
            Move::Pass => {
                self.next_player = self.next_player.other();
                self.update_state_passed();
            }
            Move::Place(tile) => {
                let curr = self.next_player;
                let prev_tile = self.tile_mut(tile).replace(curr);
                assert_eq!(prev_tile, None);

                let capture = self.clear(curr.other());
                let suicide = self.clear(curr);

                // source for this assert: http://webdocs.cs.ualberta.ca/~hayward/396/hoven/tromptaylor.pdf
                // TODO maybe just skip clearing curr instead for performance
                if capture {
                    assert!(!suicide);
                }

                self.next_player = self.next_player.other();
                self.state = State::Normal;
            }
        }

        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        match self.state {
            State::Normal | State::Passed => None,
            State::Done(outcome) => Some(outcome),
        }
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl<'a> BoardMoves<'a, GoBoard> for GoBoard {
    type AllMovesIterator = AllMovesIterator<GoBoard>;
    type AvailableMovesIterator = AvailableMovesIterator<'a, GoBoard>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        AllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        AvailableMovesIterator::new(self)
    }
}

impl InternalIterator for AllMovesIterator<GoBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        f(Move::Pass)?;
        for tile in Tile::all(GoBoard::MAX_SIZE) {
            f(Move::Place(tile))?;
        }
        ControlFlow::Continue(())
    }
}

impl InternalIterator for AvailableMovesIterator<'_, GoBoard> {
    type Item = Move;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let board = self.board();

        // TODO remove repeating moves

        // we already know the board is not done at this point
        //  so we can just yield all empty tiles (and the pass move)
        f(Move::Pass)?;
        for tile in Tile::all(board.size) {
            if board.tile(tile).is_none() {
                f(Move::Place(tile))?;
            }
        }
        ControlFlow::Continue(())
    }

    fn count(self) -> usize {
        let board = self.board();

        // TODO remove repeating moves
        1 + board.tiles.iter().filter(|t| t.is_none()).count()
    }
}

// TODO implement proper symmetry
impl_unit_symmetry_board!(GoBoard);

impl Debug for GoBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GoBoard({:?})", self.to_fen())
    }
}

impl Display for GoBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self)?;

        for y in (0..self.size).rev() {
            write!(f, "{:2} ", y + 1)?;

            for x in 0..self.size {
                let tile = Tile::new(x, y);
                let player = self.tile(tile);
                let c = match player {
                    None => {
                        let reaches_a = self.reaches(tile, Some(Player::A));
                        let reaches_b = self.reaches(tile, Some(Player::B));
                        if reaches_a ^ reaches_b {
                            '+'
                        } else {
                            '.'
                        }
                    }
                    Some(player) => player_symbol(player),
                };
                write!(f, "{}", c)?;
            }

            if y == self.size / 2 {
                write!(
                    f,
                    "    {}  {:?}  {:?}",
                    player_symbol(self.next_player),
                    self.state,
                    self.current_score()
                )?;
            }

            writeln!(f)?;
        }

        write!(f, "   ")?;
        for x in 0..self.size {
            write!(f, "{}", (x + b'a') as char)?;
        }
        writeln!(f)?;

        Ok(())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Pass => write!(f, "PASS"),
            Move::Place(tile) => write!(f, "{}{}", (b'A' + tile.x) as char, tile.y + 1),
        }
    }
}

fn player_symbol(player: Player) -> char {
    match player {
        Player::A => 'b',
        Player::B => 'w',
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidFen;

impl GoBoard {
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        if self.size == 0 {
            fen.push('/');
        } else {
            for y in (0..self.size).rev() {
                for x in 0..self.size {
                    let tile = Tile::new(x, y);
                    let player = self.tile(tile);
                    let c = match player {
                        None => '.',
                        Some(player) => player_symbol(player),
                    };
                    fen.push(c);
                }
                if y != 0 {
                    fen.push('/');
                }
            }
        }

        write!(&mut fen, " {}", player_symbol(self.next_player)).unwrap();

        let pass_counter = match self.state {
            State::Normal => 0,
            State::Passed => 1,
            State::Done(_) => 2,
        };
        write!(&mut fen, " {}", pass_counter).unwrap();

        fen
    }

    pub fn from_fen(fen: &str, rules: Rules) -> Result<GoBoard, InvalidFen> {
        let (tiles, next, pass) = fen.split(' ').collect_tuple().ok_or(InvalidFen)?;

        check(tiles.chars().all(|c| "/wb.".contains(c)))?;

        let mut board = if tiles == "/" {
            GoBoard::new(0, rules)
        } else {
            let lines: Vec<&str> = tiles.split('/').collect_vec();
            let size = lines.len();
            check(size <= Self::MAX_SIZE as usize)?;

            let mut board = GoBoard::new(size as u8, rules);
            for (y_rev, line) in lines.iter().enumerate() {
                let y = size - 1 - y_rev;
                check(line.len() == size)?;

                for (x, value) in line.chars().enumerate() {
                    let value = match value {
                        'b' => Some(Player::A),
                        'w' => Some(Player::B),
                        '.' => None,
                        _ => unreachable!(),
                    };
                    *board.tile_mut(Tile::new(x as u8, y as u8)) = value;
                }
            }
            board
        };

        board.next_player = match next {
            "b" => Player::A,
            "w" => Player::B,
            _ => return Err(InvalidFen),
        };

        match pass {
            "0" => board.state = State::Normal,
            "1" => board.state = State::Passed,
            "2" => {
                // set to passed once then pass again
                board.state = State::Passed;
                board.update_state_passed();
            }
            _ => return Err(InvalidFen),
        }

        Ok(board)
    }
}

fn check(c: bool) -> Result<(), InvalidFen> {
    match c {
        true => Ok(()),
        false => Err(InvalidFen),
    }
}
