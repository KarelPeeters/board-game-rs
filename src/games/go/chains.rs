use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use itertools::Itertools;

use crate::board::Player;
use crate::games::go::link::{LinkHead, LinkNode, NodeStorage, NodeStorageMut};
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{Score, Zobrist};

// TODO replace Option<u16> with NonMaxU16 everywhere

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow
#[derive(Clone)]
pub struct Chains {
    size: u8,

    tiles: Vec<Content>,
    groups: Vec<Group>,

    // derived data
    empty_list: LinkHead,
    zobrist: Zobrist,
}

// TODO compact into single u8
// TODO store the current tile in the content too without the extra indirection?
// TODO find a better name than "Content"
#[derive(Debug, Clone)]
pub struct Content {
    pub group_id: Option<u16>,
    pub link: LinkNode,
}

// TODO compact? we can at least force player into one of the other fields
// TODO do even even need player here if we also store the player in the tile itself?
#[derive(Debug, Clone)]
pub struct Group {
    pub color: Player,
    /// The stones that are part of this group.
    pub stones: LinkHead,
    /// The number of edges adjacent to to liberties.
    /// This forms an upper bound on the true number of liberties.
    /// This is easier to compute incrementally but still enough to know if a group is dead.
    pub liberty_edge_count: u16,
    // TODO also track the real liberty count?
    //   not necessary for correctness, just for heuristics and convenience
    /// The combined hash of all stones in this group.
    /// Used to quickly remove the entire group from the hash.
    pub zobrist: Zobrist,
}

// TODO replace vecs with on-stack vecs
#[derive(Debug, Clone)]
pub struct PreparedPlacement {
    pub kind: PlacementKind,

    pub new_group_stone_count: u16,
    pub new_group_liberty_edge_count_before_capture: u16,

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

macro_rules! storage {
    (&$self:expr) => {
        TileNodeStorage(&$self.tiles)
    };
    (&mut$self:expr) => {
        &mut TileNodeStorageMut(&mut $self.tiles)
    };
}

impl Chains {
    pub const MAX_SIZE: u8 = 19;
    pub const MAX_AREA: u16 = Self::MAX_SIZE as u16 * Self::MAX_SIZE as u16;

    pub fn new(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE);

        let area = size as u16 * size as u16;
        let tiles = (0..area)
            .map(|i| Content {
                group_id: None,
                link: LinkNode::full(area, i),
            })
            .collect_vec();

        Chains {
            size,
            empty_list: LinkHead::full(area),
            tiles,
            groups: vec![],
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
        self.area() - self.empty_count()
    }

    pub fn empty_count(&self) -> u16 {
        self.empty_list.len()
    }

    // TODO find a batter way to expose internal state
    //   eg. do we want to expose the group id?
    //   how can a user iterate over the stones in a group?
    pub fn content_at(&self, tile: Tile) -> &Content {
        &self.tiles[tile.index(self.size)]
    }

    pub fn group_at(&self, tile: Tile) -> Option<&Group> {
        self.content_at(tile).group_id.map(|id| &self.groups[id as usize])
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.group_at(tile).map(|group| group.color)
    }

    pub fn zobrist(&self) -> Zobrist {
        self.zobrist
    }

    /// Iterator over all of the groups that currently exist.
    /// The items are `(group_id, group)`. `group_id` is not necessarily continuous.
    pub fn groups(&self) -> impl Iterator<Item = (u16, &Group)> + '_ {
        // TODO implement exact size iterator for this? we can cache the number of alive groups
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, group)| !group.is_dead())
            .map(|(id, group)| (id as u16, group))
    }

    pub fn group_stones(&self, id: u16) -> Option<impl ExactSizeIterator<Item = Tile> + '_> {
        let group = self.groups.get(id as usize)?;
        if group.is_dead() {
            return None;
        };
        let iter = group
            .stones
            .iter(storage!(&self))
            .map(move |index| Tile::from_index(index as usize, self.size));
        Some(iter)
    }

    pub fn tile_storage(&self) -> impl NodeStorage + '_ {
        storage!(&self)
    }

    pub fn empty_tiles(&self) -> impl ExactSizeIterator<Item = Tile> + '_ {
        let size = self.size();
        self.empty_list
            .iter(storage!(&self))
            .map(move |index| Tile::from_index(index as usize, size))
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

    // TODO reorder functions
    fn allocate_group(&mut self, new: Group) -> u16 {
        match self.groups.iter().position(|g| g.is_dead()) {
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
            new_group_stone_count,
            new_group_liberty_edge_count_before_capture,
            merge_friendly,
            clear_enemy,
        } = prepared;
        let size = self.size();
        let tile_index = tile.index(size) as u16;

        match kind {
            PlacementKind::Normal | PlacementKind::Capture => {
                // update new tile
                //   the group id will be set later when updating all stones in the friendly group
                let tile_zobrist = Zobrist::for_color_tile(color, tile, size);
                self.zobrist ^= tile_zobrist;
                self.empty_list.remove(tile_index, storage!(&mut self));

                // build new merged group
                //   do this before modifying liberties so they're immediately counted for the new group
                let new_group_id = self.build_merged_group(
                    tile,
                    color,
                    &merge_friendly,
                    new_group_liberty_edge_count_before_capture,
                );
                debug_assert_eq!(new_group_stone_count, self.groups[new_group_id as usize].stones.len());

                // remove liberties from stones adjacent to tile
                change_liberty_edges_at(size, &mut self.tiles, &mut self.groups, tile, -1, Some(new_group_id));

                // remove cleared groups
                for clear_group_id in clear_enemy {
                    self.clear_group(clear_group_id);
                }
            }
            PlacementKind::SuicideSingle => {
                // don't do anything, we don't even need to place the stone
            }
            PlacementKind::SuicideMulti => {
                for clear_group_id in merge_friendly {
                    self.clear_group(clear_group_id);
                }
            }
        }

        Ok(kind)
    }

    fn build_merged_group(
        &mut self,
        tile: Tile,
        color: Player,
        merge_friendly: &[u16],
        new_group_liberty_edge_count_before_capture: u16,
    ) -> u16 {
        let size = self.size();
        let tile_index = tile.index(size);

        // track stats for new group
        let mut new_group_stones = LinkHead::single(tile_index as u16);
        let mut new_group_zobrist = Zobrist::for_color_tile(color, tile, size);

        // merge in other groups
        for &merge_group_id in merge_friendly {
            let merge_group = &mut self.groups[merge_group_id as usize];

            new_group_stones.splice_front_take(&mut merge_group.stones, storage!(&mut self));
            new_group_zobrist ^= merge_group.zobrist;

            merge_group.mark_dead();
        }

        // allocate new group
        let new_group_id = self.allocate_group(Group {
            color,
            stones: new_group_stones.clone(),
            liberty_edge_count: new_group_liberty_edge_count_before_capture,
            zobrist: new_group_zobrist,
        });

        // mark tiles as part of new group
        new_group_stones.for_each_mut(storage!(&mut self), |storage, tile_index| {
            let tiles = &mut storage.0;
            tiles[tile_index as usize].group_id = Some(new_group_id);
        });

        new_group_id
    }

    fn clear_group(&mut self, group_id: u16) {
        let size = self.size();
        let clear_group = &mut self.groups[group_id as usize];

        // remove group from global state
        self.zobrist ^= clear_group.zobrist;

        // fix per-tile state
        //  unfortunately we have to do some borrowing trickery
        {
            let stones = clear_group.stones.clone();
            let tiles = &mut self.tiles;
            let groups = &mut self.groups;

            stones.for_each_mut(&mut TileNodeStorageMut(tiles), |storage, tile_index| {
                // map params
                let tiles = &mut storage.0;
                let tile_index = tile_index as usize;
                let tile = Tile::from_index(tile_index, size);

                // remove stone->group link
                tiles[tile_index].group_id = None;

                // increase liberties of surrounding groups
                //    we might accidentally increment old group liberties here, but that shouldn't be a problem
                change_liberty_edges_at(size, tiles, groups, tile, 1, None);
            });
        }

        let clear_group = &mut self.groups[group_id as usize];

        // add all stones to empty list
        self.empty_list
            .splice_front_take(&mut clear_group.stones, storage!(&mut self));

        // mark group as dead
        clear_group.mark_dead();
    }

    pub fn simulate_place_stone(&self, tile: Tile, color: Player) -> Result<SimulatedPlacement, TileOccupied> {
        let prepared = self.prepare_place_stone(tile, color)?;
        let PreparedPlacement {
            kind,
            new_group_stone_count: _,
            new_group_liberty_edge_count_before_capture: _,
            merge_friendly,
            clear_enemy,
        } = prepared;
        let size = self.size;

        let (tile_survives, removed_groups): (bool, &[u16]) = match kind {
            PlacementKind::Normal => (true, &[]),
            PlacementKind::Capture => (true, &clear_enemy),
            PlacementKind::SuicideSingle => (false, &[]),
            PlacementKind::SuicideMulti => (false, &merge_friendly),
        };

        let mut zobrist_next = self.zobrist;
        let mut stone_count_next = self.stone_count();

        if tile_survives {
            zobrist_next ^= Zobrist::for_color_tile(color, tile, size);
            stone_count_next += 1;
        }
        for &group_id in removed_groups {
            let group = &self.groups[group_id as usize];
            zobrist_next ^= group.zobrist;
            stone_count_next -= group.stones.len();
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
    #[allow(clippy::collapsible_else_if)]
    pub fn prepare_place_stone(&self, tile: Tile, color: Player) -> Result<PreparedPlacement, TileOccupied> {
        let size = self.size;
        let tile_index = tile.index(size);
        let content = &self.tiles[tile_index];
        if content.group_id.is_some() {
            return Err(TileOccupied);
        }

        let all_adjacent = tile.all_adjacent(size);

        // investigate adjacent tiles
        let mut new_group_stone_count = 1;
        // TODO this name is a tragedy
        let mut new_group_liberty_edge_count_before_capture = 0;
        let mut new_group_zobrist = Zobrist::for_color_tile(color, tile, size);

        // TODO get rid of adjacent_groups, it's just redundant with enemy and friendly
        let mut adjacent_groups = vec![];
        let mut clear_enemy = vec![];
        let mut merge_friendly = vec![];

        for adj in all_adjacent {
            let content = self.content_at(adj);

            match content.group_id {
                None => new_group_liberty_edge_count_before_capture += 1,
                Some(group_id) => {
                    let group = &self.groups[group_id as usize];

                    adjacent_groups.push(group_id);
                    let group_adjacency_count = adjacent_groups.iter().filter(|&&c| c == group_id).count() as u16;

                    if group.color == color {
                        if group_adjacency_count == 1 {
                            new_group_stone_count += group.stones.len();
                            new_group_liberty_edge_count_before_capture += group.liberty_edge_count;
                            new_group_zobrist ^= group.zobrist;
                            merge_friendly.push(group_id);
                        }
                        new_group_liberty_edge_count_before_capture -= 1;
                    } else {
                        debug_assert!(group.liberty_edge_count >= group_adjacency_count);
                        if group.liberty_edge_count == group_adjacency_count {
                            clear_enemy.push(group_id);
                        }
                    }
                }
            }
        }

        // check that things are unique
        debug_assert!(merge_friendly.iter().dedup().count() == merge_friendly.len());
        debug_assert!(clear_enemy.iter().dedup().count() == clear_enemy.len());

        // decide what kind of placement this is
        let kind = if !clear_enemy.is_empty() {
            PlacementKind::Capture
        } else if new_group_liberty_edge_count_before_capture == 0 {
            if new_group_stone_count == 1 {
                PlacementKind::SuicideSingle
            } else {
                PlacementKind::SuicideMulti
            }
        } else {
            PlacementKind::Normal
        };

        Ok(PreparedPlacement {
            new_group_stone_count,
            new_group_liberty_edge_count_before_capture,
            merge_friendly,
            clear_enemy,
            kind,
        })
    }

    pub fn assert_valid(&self) {
        let size = self.size();

        // check per-tile stuff and collect info
        let mut group_info = HashMap::new();
        let mut empty_tiles = HashSet::new();
        let mut stone_count = 0;

        for tile in Tile::all(size) {
            let content = self.content_at(tile);

            if let Some(id) = content.group_id {
                // group must must exist
                assert!((id as usize) < self.groups.len());
                let group = &self.groups[id as usize];

                // group must be alive
                assert!(!group.is_dead());

                // track info
                let group_zobrist = group_info.entry(id).or_insert((Zobrist::default(), HashSet::default()));
                group_zobrist.0 ^= Zobrist::for_color_tile(group.color, tile, size);
                group_zobrist.1.insert(tile.index(size) as u16);

                stone_count += 1;
            } else {
                empty_tiles.insert(tile.index(size) as u16);
            }
        }

        assert_eq!(self.stone_count(), stone_count);

        // check per-group stuff
        for (id, group) in self.groups.iter().enumerate() {
            // stone_count and liberty_edge_count must agree on whether the group is dead
            assert_eq!(group.stones.is_empty(), (group.liberty_edge_count == 0));

            // groups must be used xor dead
            let is_dead = group.stones.is_empty();
            assert_ne!(group_info.contains_key(&(id as u16)), is_dead);

            let linked_stones = group.stones.assert_valid_and_collect(storage!(&self));

            // group zobrist must be correct
            if let Some(&(zobrist, ref stones)) = group_info.get(&(id as u16)) {
                assert_eq!(zobrist, group.zobrist);
                assert_eq!(stones, &linked_stones);
            } else {
                assert_eq!(Zobrist::default(), group.zobrist);
                assert!(linked_stones.is_empty());
            }
        }

        // check hash validness
        let mut new_zobrist = Zobrist::default();
        for tile in Tile::all(size) {
            if let Some(player) = self.stone_at(tile) {
                let value = Zobrist::for_color_tile(player, tile, size);
                new_zobrist ^= value;
            }
        }
        assert_eq!(self.zobrist, new_zobrist, "Invalid zobrist hash");

        // check empty tiles linkedlist
        let linked_empty_tiles = self.empty_list.assert_valid_and_collect(TileNodeStorage(&self.tiles));
        assert_eq!(empty_tiles, linked_empty_tiles);
    }
}

// This is not a function on the struct because we need to use it while things are partially borrowed.
fn change_liberty_edges_at(
    size: u8,
    tiles: &mut [Content],
    groups: &mut [Group],
    tile: Tile,
    delta: i16,
    skip_group_id: Option<u16>,
) {
    for adj in tile.all_adjacent(size) {
        if let Some(group_id) = tiles[adj.index(size)].group_id {
            if Some(group_id) != skip_group_id {
                let count = &mut groups[group_id as usize].liberty_edge_count;
                *count = count.wrapping_add_signed(delta);
            }
        }
    }
}

impl Group {
    fn mark_dead(&mut self) {
        debug_assert!(self.stones.is_empty());
        self.liberty_edge_count = 0;
        self.zobrist = Zobrist::default();
    }

    fn is_dead(&self) -> bool {
        self.stones.is_empty()
    }
}

impl Eq for Chains {}

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
    pub fn is_suicide(self) -> bool {
        match self {
            PlacementKind::Normal | PlacementKind::Capture => false,
            PlacementKind::SuicideSingle | PlacementKind::SuicideMulti => true,
        }
    }

    pub fn removes_existing_stones(self) -> bool {
        match self {
            PlacementKind::Normal | PlacementKind::SuicideSingle => false,
            PlacementKind::Capture | PlacementKind::SuicideMulti => true,
        }
    }
}

struct TileNodeStorage<'a>(&'a [Content]);

struct TileNodeStorageMut<'a>(&'a mut [Content]);

impl NodeStorage for TileNodeStorage<'_> {
    fn get_link(&self, index: u16) -> &LinkNode {
        &self.0[index as usize].link
    }
}

impl NodeStorage for TileNodeStorageMut<'_> {
    fn get_link(&self, index: u16) -> &LinkNode {
        &self.0[index as usize].link
    }
}

impl NodeStorageMut for TileNodeStorageMut<'_> {
    fn get_link_mut(&mut self, index: u16) -> &mut LinkNode {
        &mut self.0[index as usize].link
    }
}
