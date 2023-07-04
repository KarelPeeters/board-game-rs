use crate::games::go::GO_MAX_SIZE;
use crate::symmetry::D4Symmetry;
use crate::util::iter::IterExt;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Tile {
    x: u8,
    y: u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FlatTile {
    index: u16,
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
        assert!(
            x <= GO_MAX_SIZE && y <= GO_MAX_SIZE,
            "Coordinates ({}, {}) too large, max={}",
            x,
            y,
            GO_MAX_SIZE,
        );
        Tile { x, y }
    }

    pub fn to_flat(self, size: u8) -> FlatTile {
        assert!(size <= GO_MAX_SIZE);
        FlatTile::new(size as u16 * self.y as u16 + self.x as u16)
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn all(size: u8) -> impl Iterator<Item = Tile> {
        (0..size).flat_map(move |y| (0..size).map(move |x| Tile::new(x, y)))
    }

    pub fn all_adjacent(self, size: u8) -> impl Iterator<Item = Tile> + Clone {
        Direction::ALL
            .iter()
            .filter_map(move |&dir| self.adjacent_in(dir, size))
    }

    pub fn adjacent_in(&self, dir: Direction, size: u8) -> Option<Tile> {
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

    #[must_use]
    pub fn map_symmetry(&self, sym: D4Symmetry, size: u8) -> Tile {
        let (x, y) = sym.map_xy(self.x(), self.y(), size);
        Tile::new(x, y)
    }
}

impl FlatTile {
    pub fn new(index: u16) -> Self {
        FlatTile { index }
    }

    pub fn to_tile(self, size: u8) -> Tile {
        assert!(size <= GO_MAX_SIZE);
        Tile::new((self.index % size as u16) as u8, (self.index / size as u16) as u8)
    }

    pub fn index(self) -> u16 {
        self.index
    }

    pub fn all(size: u8) -> impl Iterator<Item = FlatTile> {
        let area = (size as u16) * (size as u16);
        (0..area).pure_map(|index| FlatTile { index })
    }

    // TODO check if this gets unrolled in optimized code
    pub fn all_adjacent(self, size: u8) -> impl Iterator<Item = FlatTile> + Clone {
        Direction::ALL
            .iter()
            .filter_map(move |&dir| self.adjacent_in(dir, size))
    }

    pub fn all_adjacent_opt(self, size: u8) -> impl Iterator<Item = Option<FlatTile>> + Clone {
        Direction::ALL.iter().pure_map(move |&dir| self.adjacent_in(dir, size))
    }

    // TODO optimize this? maybe with some fancier FlatTile representation?
    pub fn adjacent_in(self, dir: Direction, size: u8) -> Option<FlatTile> {
        let index = match dir {
            Direction::Up => self.index.checked_add(size as u16)?,
            Direction::Down => self.index.checked_sub(size as u16)?,
            Direction::Left => {
                if self.index % size as u16 == 0 {
                    return None;
                }
                self.index.checked_sub(1)?
            }
            Direction::Right => {
                let after = self.index.checked_add(1)?;
                if after % size as u16 == 0 {
                    return None;
                }
                after
            }
        };

        if index < (size as u16) * (size as u16) {
            Some(FlatTile { index })
        } else {
            None
        }
    }
}
