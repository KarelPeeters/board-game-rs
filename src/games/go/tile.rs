#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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

    // TODO return u16 instead?
    pub fn index(&self, size: u8) -> usize {
        self.y as usize * size as usize + self.x as usize
    }

    pub fn from_index(index: usize, size: u8) -> Tile {
        assert!(index < size as usize * size as usize);
        Tile::new((index % size as usize) as u8, (index / size as usize) as u8)
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
}
