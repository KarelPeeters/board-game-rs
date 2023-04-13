use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use itertools::Itertools;

use crate::board::Player;
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{Score, Zobrist};

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow
#[derive(Clone, Eq)]
pub struct Chains {
    size: u8,

    // core storage
    first_empty: Option<u16>,
    last_empty: Option<u16>,

    tiles: Vec<Content>,
    groups: Vec<Group>,

    // derived data
    stone_count: u16,
    zobrist: Zobrist,
}

// TODO compact into single u8
// TODO store the current tile in the content too without the extra indirection?
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Content {
    pub group_id: Option<u16>,
    pub next_empty: Option<u16>,
    pub prev_empty: Option<u16>,
}

// TODO compact? we can at least force player into one of the other fields
// TODO do even even need player here if we also store the player in the tile itself?
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Group {
    pub color: Player,
    pub stone_count: u16,
    /// The number of edges adjacent to to liberties.
    /// This forms an upper bound on the true number of liberties.
    /// This is easier to compute incrementally but still enough to know if a group is dead.
    pub liberty_edge_count: u16,
    // TODO also track the real liberty count?
    //   not necessary for correctness, just for heuristics and convenience
    // TODO add hash to group so we can quickly remove the entire group
}

// TODO replace vecs with on-stack vecs
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PreparedPlacement {
    pub kind: PlacementKind,
    pub new_group: Group,
    pub merge_friendly: Vec<u16>,
    pub clear_enemy: Vec<u16>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SimulatedPlacement {
    pub kind: PlacementKind,
    // TODO remove next from names?
    pub zobrist_next: Zobrist,
    pub stone_count_next: u16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlacementKind {
    Normal,
    Capture,
    // TODO merge with count field?
    SuicideSingle,
    SuicideMulti,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TileOccupied;

impl Chains {
    pub const MAX_SIZE: u8 = 19;
    pub const MAX_AREA: u16 = Self::MAX_SIZE as u16 * Self::MAX_SIZE as u16;

    pub fn new(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE);

        let area = size as u16 * size as u16;
        let tiles = (0..area)
            .map(|i| Content {
                group_id: None,
                next_empty: if i + 1 < area { Some(i + 1) } else { None },
                prev_empty: if i > 0 { Some(i - 1) } else { None },
            })
            .collect_vec();

        Chains {
            size,
            first_empty: Some(0),
            last_empty: Some(area - 1),
            tiles,
            groups: vec![],
            stone_count: 0,
            zobrist: Zobrist::default(),
        }
    }

    pub fn size(&self) -> u8 {
        self.size
    }

    pub fn area(&self) -> u16 {
        self.size as u16 * self.size as u16
    }

    pub fn stone_count(&self) -> u16 {
        self.stone_count
    }

    pub fn empty_count(&self) -> u16 {
        self.area() - self.stone_count
    }

    pub fn content_at(&self, tile: Tile) -> Content {
        self.tiles[tile.index(self.size)]
    }

    pub fn group_at(&self, tile: Tile) -> Option<Group> {
        self.content_at(tile).group_id.map(|id| self.groups[id as usize])
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.group_at(tile).map(|group| group.color)
    }

    pub fn zobrist(&self) -> Zobrist {
        self.zobrist
    }

    /// Iterator over all of the groups that currently exist.
    /// The items are `(group_id, group)`. `group_id` is not necessarily continuous.
    pub fn groups(&self) -> impl Iterator<Item = (u16, Group)> + '_ {
        self.groups
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, group)| group.stone_count != 0)
            .map(|(id, group)| (id as u16, group))
    }

    pub fn empty_tiles(&self) -> impl Iterator<Item = Tile> + '_ {
        // TODO optimize with empty-tile linked list
        // TODO also override count with the cached count?
        Tile::all(self.size()).filter(move |&tile| self.stone_at(tile).is_none())
    }

    /// Is there a path between `start` and another tile with value `target` over only `player` tiles?
    pub fn reaches(&self, start: Tile, target: Option<Player>) -> bool {
        // TODO implement more quickly with chains
        //   alternatively, keep this as a fallback for unit tests
        let through = self.stone_at(start);
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
                if let Some(adj) = tile.adjacent_in(dir, self.size) {
                    let value = self.stone_at(adj);
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

    pub fn score(&self) -> Score {
        // TODO rewrite using chains
        // TODO maybe even move to chains?

        let mut score_a = 0;
        let mut score_b = 0;

        for tile in Tile::all(self.size()) {
            match self.stone_at(tile) {
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

    fn allocate_group(&mut self, new: Group) -> u16 {
        match self.groups.iter().position(|g| g.stone_count == 0) {
            Some(id) => {
                self.groups[id] = new;
                id as u16
            }
            None => {
                let id = self.groups.len() as u16;
                self.groups.push(new);
                id
            }
        }
    }

    pub fn place_stone(&mut self, tile: Tile, color: Player) -> Result<PlacementKind, TileOccupied> {
        let prepared = self.prepare_place_stone(tile, color)?;
        let PreparedPlacement {
            kind,
            new_group,
            merge_friendly,
            clear_enemy,
        } = prepared;

        match kind {
            PlacementKind::Normal | PlacementKind::Capture => {
                if merge_friendly.len() <= 1 && clear_enemy.is_empty() {
                    // fast case: no actual merging or clearing necessary
                    // TODO consider removing this once update_tile_groups is optimized properly
                    if let Some(&group_id) = merge_friendly.first() {
                        // reuse existing friendly group
                        self.set_stone_at(tile, color, group_id);
                        self.groups[group_id as usize] = new_group;
                    } else {
                        // allocate new friendly group
                        let new_group_id = self.allocate_group(new_group);
                        self.set_stone_at(tile, color, new_group_id);
                    }
                } else {
                    let new_group_id = self.allocate_group(new_group);
                    self.set_stone_at(tile, color, new_group_id);
                    self.update_tile_groups(&clear_enemy, color.other(), &merge_friendly, new_group_id);
                }
            }
            PlacementKind::SuicideSingle => {
                // don't do anything, we don't even need to place the stone
            }
            PlacementKind::SuicideMulti => {
                // we don't need to actually place the stone
                //   clear the merged friendly groups, don't merge anything
                self.update_tile_groups(&merge_friendly, color, &[], u16::MAX);
            }
        }

        Ok(kind)
    }

    pub fn simulate_place_stone(&self, tile: Tile, color: Player) -> Result<SimulatedPlacement, TileOccupied> {
        let prepared = self.prepare_place_stone(tile, color)?;
        let PreparedPlacement {
            kind,
            new_group: _,
            merge_friendly,
            clear_enemy,
        } = prepared;
        let size = self.size;

        let (tile_survives, removed_groups_color) = match kind {
            PlacementKind::Normal => (true, None),
            PlacementKind::Capture => (true, Some((clear_enemy, color.other()))),
            PlacementKind::SuicideSingle => (false, None),
            PlacementKind::SuicideMulti => (false, Some((merge_friendly, color))),
        };

        let mut zobrist_next = self.zobrist;
        let mut stone_count_next = self.stone_count;

        if tile_survives {
            zobrist_next ^= Zobrist::for_player_tile(color, tile, size);
            stone_count_next += 1;
        }
        if let Some((removed_groups, removed_color)) = removed_groups_color {
            // TODO use per-group cached zobrist instead
            for other in Tile::all(size) {
                if let Some(group_id) = self.tiles[other.index(size)].group_id {
                    if removed_groups.contains(&group_id) {
                        zobrist_next ^= Zobrist::for_player_tile(removed_color, other, size);
                        stone_count_next -= 1;
                    }
                }
            }
        }

        Ok(SimulatedPlacement {
            kind,
            zobrist_next,
            stone_count_next,
        })
    }

    // TODO unroll this whole thing into the 4 directions?
    //    collected inputs: type/group_id/liberties for each size
    //    outputs: what groups to merge and what groups to kill
    //    => simple function with 4 inputs, 4 outputs
    pub fn prepare_place_stone(&self, tile: Tile, color: Player) -> Result<PreparedPlacement, TileOccupied> {
        let size = self.size;
        let content = self.tiles[tile.index(size)];
        if content.group_id.is_some() {
            return Err(TileOccupied);
        }

        let all_adjacent = tile.all_adjacent(size);

        // investigate adjacent tiles
        let mut new_group = Group {
            color: color,
            stone_count: 1,
            liberty_edge_count: 0,
        };

        let mut adjacent_groups = vec![];
        let mut clear_enemy = vec![];
        let mut merge_friendly = vec![];

        for adj in all_adjacent.clone() {
            let content = self.content_at(adj);

            match content.group_id {
                None => new_group.liberty_edge_count += 1,
                Some(group_id) => {
                    let group = self.groups[group_id as usize];

                    adjacent_groups.push(group_id);
                    let group_factor = adjacent_groups.iter().filter(|&&c| c == group_id).count() as u16;

                    if group.color == color {
                        if group_factor == 1 {
                            new_group.stone_count += group.stone_count;
                            new_group.liberty_edge_count += group.liberty_edge_count;
                        }
                        new_group.liberty_edge_count -= 1;
                        merge_friendly.push(group_id);
                    } else {
                        if group.liberty_edge_count == group_factor {
                            clear_enemy.push(group_id);
                        }
                    }
                }
            }
        }

        // decide what kind of placement this is
        let kind = if !clear_enemy.is_empty() {
            PlacementKind::Capture
        } else if new_group.liberty_edge_count == 0 {
            if new_group.stone_count == 1 {
                PlacementKind::SuicideSingle
            } else {
                PlacementKind::SuicideMulti
            }
        } else {
            PlacementKind::Normal
        };

        // TODO deduplicate merge_friendly and clear_enemy?

        Ok(PreparedPlacement {
            new_group,
            merge_friendly,
            clear_enemy,
            kind,
        })
    }

    fn set_stone_at(&mut self, tile: Tile, color: Player, group: u16) {
        let size = self.size();

        // update tile itself
        debug_assert!(self.stone_at(tile).is_none());
        let content = &mut self.tiles[tile.index(size)];
        content.group_id = Some(group);

        // remove from empty linked list
        {
            let prev = content.prev_empty.take();
            let next = content.next_empty.take();

            match prev {
                None => self.first_empty = next,
                Some(prev) => self.tiles[prev as usize].next_empty = next,
            }
            match next {
                None => self.last_empty = prev,
                Some(next) => self.tiles[next as usize].prev_empty = prev,
            }
        }

        // update hash and count
        self.zobrist ^= Zobrist::for_player_tile(color, tile, size);
        self.stone_count += 1;

        // decrease liberty of adjacent
        for adj in Tile::all_adjacent(tile, size) {
            if let Some(adj_group_id) = self.content_at(adj).group_id {
                self.groups[adj_group_id as usize].liberty_edge_count -= 1;
            }
        }
    }

    fn clear_stone_at(&mut self, tile: Tile, color: Player, clear: &[u16], merge: &[u16], into: u16) {
        let size = self.size();
        let tile_index = tile.index(size);

        // update tile itself
        debug_assert!(self.stone_at(tile) == Some(color));
        let content = &mut self.tiles[tile_index];
        content.group_id = None;

        // insert into empty linked list at the front
        {
            let prev_first = self.first_empty;

            assert_eq!(None, content.prev_empty);
            self.first_empty = Some(tile_index as u16);

            content.next_empty = prev_first;
            match prev_first {
                None => self.last_empty = Some(tile_index as u16),
                Some(next_empty) => self.tiles[next_empty as usize].prev_empty = Some(tile_index as u16),
            }
        }

        // update hash and count
        self.zobrist ^= Zobrist::for_player_tile(color, tile, size);
        self.stone_count -= 1;

        // increase liberty of adjacent
        for adj in Tile::all_adjacent(tile, size) {
            if let Some(old_id) = self.content_at(adj).group_id {
                let id = if merge.contains(&old_id) { into } else { old_id };
                if clear.contains(&id) {
                    continue;
                }

                self.groups[id as usize].liberty_edge_count += 1;
            }
        }
    }

    // TODO optimize this with some linked-list type thing through tiles
    fn update_tile_groups(&mut self, clear: &[u16], color: Player, merge: &[u16], into: u16) {
        // update the tiles
        for tile in Tile::all(self.size()) {
            let size = self.size();

            let content = &mut self.tiles[tile.index(size)];
            if let Some(group_id) = content.group_id {
                if clear.contains(&group_id) {
                    self.clear_stone_at(tile, color, clear, merge, into);
                } else if merge.contains(&group_id) {
                    content.group_id = Some(into);
                }
            }
        }

        // mark the cleared groups as dead
        for &clear_group_id in clear {
            let clear_group = &mut self.groups[clear_group_id as usize];
            // we can't assert the liberty count here, we never actually decrement groups adjacent to the suicide stone
            clear_group.mark_dead();
        }

        // mark the now-absolute merged groups as dead
        for &merge_group_id in merge {
            let merge_group = &mut self.groups[merge_group_id as usize];
            merge_group.mark_dead();
        }
    }

    pub fn assert_valid(&self) {
        let size = self.size();

        // check per-tile stuff and collect info
        let mut used_groups = HashSet::new();
        let mut empty_tiles = HashSet::new();
        let mut stone_count = 0;

        for tile in Tile::all(size) {
            let content = self.content_at(tile);

            if let Some(id) = content.group_id {
                // group must must exist
                assert!((id as usize) < self.groups.len());
                let group = self.groups[id as usize];

                // group must be alive
                assert!(group.liberty_edge_count > 0 && group.stone_count > 0);

                // non-empty tiles should not be part of the empty linked list
                assert_eq!(None, content.prev_empty);
                assert_eq!(None, content.next_empty);

                // track info
                used_groups.insert(id);
                stone_count += 1;
            } else {
                empty_tiles.insert(tile.index(size));
            }
        }

        assert_eq!(self.stone_count, stone_count);

        // check per-group stuff
        for (id, group) in self.groups.iter().enumerate() {
            // stone_count and liberty_edge_count must agree on whether the group is dead
            assert_eq!((group.stone_count == 0), (group.liberty_edge_count == 0));

            // groups must be used xor dead
            assert!(used_groups.contains(&(id as u16)) ^ group.is_dead());
        }

        // check hash validness
        let mut new_zobrist = Zobrist::default();
        for tile in Tile::all(size) {
            if let Some(player) = self.stone_at(tile) {
                let value = Zobrist::for_player_tile(player, tile, size);
                new_zobrist ^= value;
            }
        }
        assert_eq!(self.zobrist, new_zobrist, "Invalid zobrist hash");

        // check empty tiles linkedlist
        {
            let mut linked_empty_tiles = HashSet::new();

            // "A |-> B" is read as "A points to B but B does not point back"
            // TODO are we testing everything properly? should we also walk backwards?
            match self.first_empty {
                None => assert_eq!(None, self.last_empty, "Wrong last: start |-> end"),
                Some(first) => assert_eq!(
                    None, self.tiles[first as usize].prev_empty,
                    "Wrong prev: start |-> {first}"
                ),
            }
            let mut next = self.first_empty;

            while let Some(curr) = next {
                if !linked_empty_tiles.insert(curr as usize) {
                    panic!("Empty linked list contains loop!");
                }
                next = self.tiles[curr as usize].next_empty;
                match next {
                    None => assert_eq!(Some(curr), self.last_empty, "Wrong last: {curr} |-> end"),
                    Some(next) => assert_eq!(
                        Some(curr),
                        self.tiles[next as usize].prev_empty,
                        "Wrong prev: {curr} |-> {next}"
                    ),
                }
            }

            assert_eq!(empty_tiles, linked_empty_tiles);
        }
    }
}

impl Group {
    fn mark_dead(&mut self) {
        self.stone_count = 0;
        self.liberty_edge_count = 0;
    }

    // TODO give this a better name and clarify the semantics
    fn is_dead(&self) -> bool {
        self.stone_count == 0
    }
}

impl PartialEq for Chains {
    fn eq(&self, other: &Self) -> bool {
        self.tiles.len() == other.tiles.len()
            && self.zobrist() == other.zobrist()
            && Tile::all(self.size).all(|tile| self.stone_at(tile) == other.stone_at(tile))
    }
}

impl Hash for Chains {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zobrist().hash(state);
    }
}

// TODO move to io?
impl Debug for Chains {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chains({:?})", self.to_fen())
    }
}

// TODO move to io?
impl Display for Chains {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chains {{")?;
        writeln!(f, "  fen: {:?}", self.to_fen())?;

        writeln!(f, "  tiles:")?;
        let size = self.size();
        for y in (0..size).rev() {
            write!(f, "    {:2} ", y + 1)?;
            for x in 0..size {
                let tile = Tile::new(x, y);
                match self.tiles[tile.index(size)].group_id {
                    None => write!(f, "   .")?,
                    Some(group) => write!(f, "{:4}", group)?,
                }
            }
            writeln!(f)?;
        }
        write!(f, "       ")?;
        for x in 0..size {
            write!(f, "   {}", Tile::x_to_char(x).unwrap())?;
        }
        writeln!(f)?;

        // TODO only print alive groups?
        writeln!(f, "  groups:")?;
        for (i, group) in self.groups.iter().enumerate() {
            writeln!(f, "    group {}: {:?}", i, group)?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }
}

impl PlacementKind {
    pub fn is_suicide(&self) -> bool {
        match self {
            PlacementKind::Normal | PlacementKind::Capture => false,
            PlacementKind::SuicideSingle | PlacementKind::SuicideMulti => true,
        }
    }

    pub fn removes_existing_stones(&self) -> bool {
        match self {
            PlacementKind::Normal | PlacementKind::SuicideSingle => false,
            PlacementKind::Capture | PlacementKind::SuicideMulti => true,
        }
    }
}
