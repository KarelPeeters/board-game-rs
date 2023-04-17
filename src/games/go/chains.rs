use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use itertools::Itertools;
use rand::seq::IteratorRandom;
use rand::Rng;

use crate::board::Player;
use crate::games::go::link::{LinkHead, LinkNode};
use crate::games::go::stack_vec::StackVec4;
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{FlatTile, Linked, Score, TileX, Zobrist, GO_MAX_SIZE};
use crate::util::iter::IterExt;

// TODO replace Option<u16> with NonMaxU16 everywhere

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow

// TODO clean up getters, set right visibility

// TODO do a bunch of struct-of-array instead of array-of-struct stuff?
//   unfortunately that would require allocating more separate vecs

#[derive(Clone)]
pub struct Chains {
    size: u8,

    tiles: Vec<Content>,
    groups: Vec<Group>,

    // derived data
    stones_a: u16,
    empty_list: LinkHead,
    dead_groups: LinkHead,
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
    /// The number of edges adjacent to to liberties.
    /// This forms an upper bound on the true number of liberties.
    /// This is easier to compute incrementally but still enough to know if a group is dead.
    pub liberty_edge_count: u16,
    // TODO also track the real liberty count?
    //   not necessary for correctness, just for heuristics and convenience
    /// The combined hash of all stones in this group.
    /// Used to quickly remove the entire group from the hash.
    pub zobrist: Zobrist,

    /// The stones that are part of this group.
    pub stones: LinkHead,
    /// Link in the dead group list.
    pub dead_link: LinkNode,
}

// TODO replace pub with getters to ensure they don't get tampered with?
// TODO remove as much computation from this as possible again, it's slowing things down
//    we really only need the zobrist, leave other state for some separate simulate function
// TODO remove tile, color, counts?
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimulatedPlacement {
    pub tile: FlatTile,
    pub color: Player,
    pub kind: PlacementKind,

    // stats right after the stone is placed, before removing dead groups
    pub new_group_stone_count: u16,
    pub new_group_initial_liberty_edge_count: u16,

    // the groups that will be merged/removed
    pub merge_friendly: StackVec4,
    pub clear_enemy: StackVec4,

    // the state of the board after this move
    pub next_zobrist: Zobrist,
    pub next_stone_count: u16,
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
    pub fn new(size: u8) -> Self {
        assert!(size <= GO_MAX_SIZE);

        let area = size as u16 * size as u16;
        let tiles = (0..area)
            .map(|i| Content {
                group_id: None,
                link: LinkNode::full(area, i),
            })
            .collect_vec();

        Chains {
            size,
            tiles,
            groups: vec![],
            stones_a: 0,
            empty_list: LinkHead::full(area),
            zobrist: Zobrist::default(),
            dead_groups: LinkHead::empty(),
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

    pub fn stone_count_from(&self, player: Player) -> u16 {
        match player {
            Player::A => self.stones_a,
            Player::B => self.stone_count() - self.stones_a,
        }
    }

    pub fn empty_count(&self) -> u16 {
        self.empty_list.len()
    }

    // TODO find a batter way to expose internal state
    //   eg. do we want to expose the group id?
    //   how can a user iterate over the stones in a group?
    pub fn content_at(&self, tile: FlatTile) -> &Content {
        &self.tiles[tile.index() as usize]
    }

    pub fn group_at(&self, tile: FlatTile) -> Option<&Group> {
        self.content_at(tile).group_id.map(|id| &self.groups[id as usize])
    }

    pub fn stone_at(&self, tile: FlatTile) -> Option<Player> {
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
            .pure_map(|(id, group)| (id as u16, group))
    }

    pub fn tiles(&self) -> &[Content] {
        &self.tiles
    }

    pub fn empty_tiles(&self) -> impl ExactSizeIterator<Item = FlatTile> + '_ {
        self.empty_list.iter(&self.tiles).pure_map(FlatTile::new)
    }

    pub fn random_empty_tile(&self, rng: &mut impl Rng) -> Option<FlatTile> {
        self.random_empty_tile_where(rng, |_| true)
    }

    /// Uniformly sample an empty tile for which `f(tile)` is true.
    ///
    /// This implementation is optimized assuming `f` is very likely to return true.
    pub fn random_empty_tile_where(&self, rng: &mut impl Rng, mut f: impl FnMut(FlatTile) -> bool) -> Option<FlatTile> {
        if self.empty_list.is_empty() {
            return None;
        }

        // TODO optimize sampling coefficients, use a mix of different sizes to profile
        const FULL_SAMPLE_MIN_EMPTY: u16 = 8;
        const FULL_SAMPLE_MIN_EMPTY_FRAC: f32 = 0.2;
        const FULL_SAMPLE_TRIES: u32 = 16;
        const EMPTY_SAMPLE_TRIES: u32 = 16;

        const FRAC_DENOM: u32 = 128;
        const FRAC_NUMER: u32 = (FULL_SAMPLE_MIN_EMPTY_FRAC * FRAC_DENOM as f32) as u32;

        // if there are enough empty times, just randomly sample until we find one
        let empty_count = self.empty_count();
        let empty_per_64 = empty_count as u32 * FRAC_DENOM / self.area() as u32;
        if empty_count >= FULL_SAMPLE_MIN_EMPTY && empty_per_64 > FRAC_NUMER {
            for _ in 0..EMPTY_SAMPLE_TRIES {
                let tile = FlatTile::new(rng.gen_range(0..self.area()));
                if self.stone_at(tile).is_none() && f(tile) {
                    return Some(tile);
                }
            }
        }

        // partial fallback: sample random empty tiles and check if they match
        for _ in 0..FULL_SAMPLE_TRIES {
            let tile = self.empty_tiles().choose(rng).unwrap();
            if f(tile) {
                return Some(tile);
            }
        }

        // full fallback: sample from fully filtered list
        //   this ensures that we only return None if we have actually checked all empty tiles
        self.empty_tiles().filter(|&tile| f(tile)).choose(rng)
    }

    /// Is there a path between `start` and another tile with value `target` over only `player` tiles?
    pub fn reaches(&self, start: FlatTile, target: Option<Player>) -> bool {
        // TODO implement more quickly with chains
        //   alternatively, keep this as a fallback for unit tests
        let through = self.stone_at(start);
        assert_ne!(through, target);

        let mut visited = vec![false; self.tiles.len()];
        let mut stack = vec![start];

        while let Some(tile) = stack.pop() {
            let index = tile.index() as usize;
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

        for tile in FlatTile::all(self.size()) {
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

    pub fn place_stone(&mut self, tile: FlatTile, color: Player) -> Result<SimulatedPlacement, TileOccupied> {
        let simulated = self.simulate_place_stone(tile, color)?;
        self.apply_simulated_placement(&simulated);
        Ok(simulated)
    }

    // TODO unroll this whole thing into the 4 directions?
    //    collected inputs: type/group_id/liberties for each size
    //    outputs: what groups to merge and what groups to kill
    //    => simple function with 4 inputs, 4 outputs
    #[allow(clippy::collapsible_else_if)]
    pub fn simulate_place_stone(&self, tile: FlatTile, color: Player) -> Result<SimulatedPlacement, TileOccupied> {
        let size = self.size;
        let content = &self.tiles[tile.index() as usize];

        if content.group_id.is_some() {
            return Err(TileOccupied);
        }

        // investigate adjacent tiles
        let mut new_group_initial_liberty_edge_count = 0;
        let mut merged_zobrist = Zobrist::default();
        let mut merged_count = 0;
        let mut captured_zobrist = Zobrist::default();
        let mut captured_stone_count = 0;

        // TODO get rid of adjacent_groups, it's just redundant with enemy and friendly
        let mut adjacent_groups = StackVec4::new();
        let mut clear_enemy = StackVec4::new();
        let mut merge_friendly = StackVec4::new();

        // TODO unroll?
        for (adj_i, adj) in tile.all_adjacent(size).enumerate() {
            let content = self.content_at(adj);

            match content.group_id {
                None => new_group_initial_liberty_edge_count += 1,
                Some(group_id) => {
                    let group = &self.groups[group_id as usize];

                    adjacent_groups[adj_i] = group_id;
                    let group_adjacency_count = adjacent_groups.count(group_id);

                    if group.color == color {
                        if group_adjacency_count == 1 {
                            merged_count += group.stones.len();
                            new_group_initial_liberty_edge_count += group.liberty_edge_count;
                            merged_zobrist ^= group.zobrist;
                            merge_friendly[adj_i] = group_id;
                        }
                        new_group_initial_liberty_edge_count -= 1;
                    } else {
                        debug_assert!(group.liberty_edge_count as usize >= group_adjacency_count);
                        if group.liberty_edge_count as usize == group_adjacency_count {
                            clear_enemy[adj_i] = group_id;
                            captured_stone_count += group.stones.len();
                            captured_zobrist ^= group.zobrist;
                        }
                    }
                }
            }
        }

        debug_assert!(!merge_friendly.contains_duplicates());
        debug_assert!(!clear_enemy.contains_duplicates());

        // decide what kind of placement this is

        let kind = if !clear_enemy.is_empty() {
            PlacementKind::Capture
        } else if new_group_initial_liberty_edge_count == 0 {
            if merged_count == 0 {
                PlacementKind::SuicideSingle
            } else {
                PlacementKind::SuicideMulti
            }
        } else {
            PlacementKind::Normal
        };

        // calculate final stats
        let mut next_zobrist = self.zobrist ^ captured_zobrist;
        let mut next_stone_count = self.stone_count() - captured_stone_count;
        match kind {
            PlacementKind::Normal | PlacementKind::Capture => {
                next_zobrist ^= Zobrist::for_color_tile(color, tile);
                next_stone_count += 1;
            }
            PlacementKind::SuicideSingle => {}
            PlacementKind::SuicideMulti => {
                next_zobrist ^= merged_zobrist;
                next_stone_count -= merged_count;
            }
        }

        // TODO include new_group_zobrist?
        Ok(SimulatedPlacement {
            tile,
            color,
            kind,
            new_group_stone_count: 1 + merged_count,
            new_group_initial_liberty_edge_count,
            merge_friendly,
            clear_enemy,
            next_zobrist,
            next_stone_count,
        })
    }

    pub fn apply_simulated_placement(&mut self, simulated: &SimulatedPlacement) {
        let &SimulatedPlacement {
            tile,
            color,
            kind,
            new_group_stone_count,
            new_group_initial_liberty_edge_count,
            ref merge_friendly,
            ref clear_enemy,
            next_zobrist,
            next_stone_count,
        } = simulated;

        let size = self.size();
        let tile_index = tile.index();

        match kind {
            PlacementKind::Normal | PlacementKind::Capture => {
                // update new tile
                //   the group id will be set later when updating all stones in the friendly group
                let tile_zobrist = Zobrist::for_color_tile(color, tile);
                self.zobrist ^= tile_zobrist;
                self.empty_list.remove(tile_index, &mut self.tiles);
                if color == Player::A {
                    self.stones_a += 1;
                }

                // fast case: no clearing, no real merging
                if merge_friendly.len() <= 1 && clear_enemy.is_empty() {
                    let group_id = if let Some(group_id) = merge_friendly.first() {
                        // merge into single adjacent friendly group
                        let group = &mut self.groups[group_id as usize];

                        group.zobrist ^= tile_zobrist;
                        group.stones.insert_front(tile_index, &mut self.tiles);
                        group.liberty_edge_count = new_group_initial_liberty_edge_count;

                        group_id
                    } else {
                        // no adjacent, allocate new group
                        self.allocate_group(Group {
                            color,
                            stones: LinkHead::single(tile_index),
                            liberty_edge_count: new_group_initial_liberty_edge_count,
                            zobrist: tile_zobrist,
                            dead_link: LinkNode::single(),
                        })
                    };

                    // set tile itself
                    self.tiles[tile_index as usize].group_id = Some(group_id);

                    // decrement adjacent liberties
                    change_liberty_edges_at(size, &mut self.tiles, &mut self.groups, tile, -1, Some(group_id));
                } else {
                    // build new merged group
                    //   do this before modifying liberties so they're immediately counted for the new group
                    let new_group_id =
                        self.build_merged_group(tile, color, merge_friendly, new_group_initial_liberty_edge_count);
                    debug_assert_eq!(new_group_stone_count, self.groups[new_group_id as usize].stones.len());

                    // remove liberties from stones adjacent to tile
                    change_liberty_edges_at(size, &mut self.tiles, &mut self.groups, tile, -1, Some(new_group_id));

                    // remove cleared groups
                    clear_enemy.for_each(|clear_group_id| {
                        self.clear_group(clear_group_id);
                    });
                }
            }
            PlacementKind::SuicideSingle => {
                // don't do anything, we don't even need to place the stone
            }
            PlacementKind::SuicideMulti => {
                merge_friendly.for_each(|clear_group_id| {
                    self.clear_group(clear_group_id);
                });
            }
        }

        debug_assert_eq!(self.zobrist, next_zobrist);
        debug_assert_eq!(self.stone_count(), next_stone_count);
    }

    // TODO merge into largest existing merged group so we can skip changing those tiles?
    fn build_merged_group(
        &mut self,
        tile: FlatTile,
        color: Player,
        merge_friendly: &StackVec4,
        new_group_initial_liberty_edge_count: u16,
    ) -> u16 {
        let tile_index = tile.index();

        // track stats for new group
        let mut new_group_stones = LinkHead::single(tile_index);
        let mut new_group_zobrist = Zobrist::for_color_tile(color, tile);

        // merge in other groups
        merge_friendly.for_each(|merge_group_id| {
            let merge_group = &mut self.groups[merge_group_id as usize];

            new_group_stones.splice_front_take(&mut merge_group.stones, &mut self.tiles);
            new_group_zobrist ^= merge_group.zobrist;

            self.free_group(merge_group_id);
        });

        // allocate new group
        let new_group_id = self.allocate_group(Group {
            color,
            stones: new_group_stones.clone(),
            liberty_edge_count: new_group_initial_liberty_edge_count,
            zobrist: new_group_zobrist,
            dead_link: LinkNode::single(),
        });

        // mark tiles as part of new group
        new_group_stones.for_each_mut(&mut self.tiles, |tiles, tile_index| {
            tiles[tile_index as usize].group_id = Some(new_group_id);
        });

        new_group_id
    }

    fn clear_group(&mut self, group_id: u16) {
        let size = self.size();
        let clear_group = &mut self.groups[group_id as usize];

        // remove group from global state
        self.zobrist ^= clear_group.zobrist;
        if clear_group.color == Player::A {
            self.stones_a -= clear_group.stones.len();
        }

        // fix per-tile state
        //  unfortunately we have to do some borrowing trickery
        {
            let stones = clear_group.stones.clone();
            let tiles = &mut self.tiles;
            let groups = &mut self.groups;

            stones.for_each_mut(tiles, |tiles, tile_index| {
                // map params
                let tile = FlatTile::new(tile_index);

                // remove stone->group link
                tiles[tile_index as usize].group_id = None;

                // increase liberties of surrounding groups
                //    we might accidentally increment old group liberties here, but that shouldn't be a problem
                change_liberty_edges_at(size, tiles, groups, tile, 1, None);
            });
        }

        let clear_group = &mut self.groups[group_id as usize];

        // add all stones to empty list
        self.empty_list
            .splice_front_take(&mut clear_group.stones, &mut self.tiles);

        // mark group as dead
        self.free_group(group_id);
    }

    fn allocate_group(&mut self, new: Group) -> u16 {
        match self.dead_groups.pop_front(&mut self.groups) {
            Some(id) => {
                self.groups[id as usize] = new;
                id
            }
            None => {
                let id = self.groups.len() as u16;
                self.groups.push(new);
                id
            }
        }
    }

    fn free_group(&mut self, id: u16) {
        let group = &mut self.groups[id as usize];
        debug_assert!(group.stones.is_empty());
        debug_assert!(group.dead_link.is_unconnected_or_single());

        // mark group itself as dead
        self.groups[id as usize] = Group {
            color: group.color,
            stones: LinkHead::empty(),
            liberty_edge_count: 0,
            zobrist: Default::default(),
            dead_link: LinkNode::single(),
        };

        // insert into empty list
        self.dead_groups.insert_front(id, &mut self.groups);
    }

    pub fn assert_valid(&self) {
        let size = self.size();

        // check per-tile stuff and collect info
        let mut group_info = HashMap::new();
        let mut empty_tiles = HashSet::new();
        let mut stone_count = 0;
        let mut stone_count_a = 0;
        let mut stone_count_b = 0;

        for tile in FlatTile::all(size) {
            let content = self.content_at(tile);

            if let Some(id) = content.group_id {
                // group must must exist
                assert!((id as usize) < self.groups.len());
                let group = &self.groups[id as usize];

                // group must be alive
                assert!(!group.is_dead());

                // track info
                let group_zobrist = group_info.entry(id).or_insert((Zobrist::default(), HashSet::default()));
                group_zobrist.0 ^= Zobrist::for_color_tile(group.color, tile);
                group_zobrist.1.insert(tile.index());

                stone_count += 1;
                match group.color {
                    Player::A => stone_count_a += 1,
                    Player::B => stone_count_b += 1,
                }
            } else {
                empty_tiles.insert(tile.index());
            }
        }

        assert_eq!(self.stone_count(), stone_count);
        assert_eq!(self.stone_count_from(Player::A), stone_count_a);
        assert_eq!(self.stone_count_from(Player::B), stone_count_b);

        let mut expected_dead_groups = HashSet::new();

        // check per-group stuff
        for (id, group) in self.groups.iter().enumerate() {
            // stone_count and liberty_edge_count must agree on whether the group is dead
            assert_eq!(group.stones.is_empty(), (group.liberty_edge_count == 0));

            // groups must be used xor dead
            let is_dead = group.stones.is_empty();
            assert_ne!(group_info.contains_key(&(id as u16)), is_dead);

            let linked_stones = group.stones.assert_valid_and_collect(&self.tiles);

            // group zobrist must be correct
            if let Some(&(zobrist, ref stones)) = group_info.get(&(id as u16)) {
                assert_eq!(zobrist, group.zobrist);
                assert_eq!(stones, &linked_stones);
            } else {
                assert_eq!(Zobrist::default(), group.zobrist);
                assert!(linked_stones.is_empty());
            }

            if group.is_dead() {
                expected_dead_groups.insert(id as u16);
            }
        }

        // check dead groups
        let linked_dead_groups = self.dead_groups.assert_valid_and_collect(&self.groups);
        assert_eq!(expected_dead_groups, linked_dead_groups);

        // check hash validness
        let mut new_zobrist = Zobrist::default();
        for tile in FlatTile::all(size) {
            if let Some(player) = self.stone_at(tile) {
                let value = Zobrist::for_color_tile(player, tile);
                new_zobrist ^= value;
            }
        }
        assert_eq!(self.zobrist, new_zobrist, "Invalid zobrist hash");

        // check empty tiles linkedlist
        let linked_empty_tiles = self.empty_list.assert_valid_and_collect(&self.tiles);
        assert_eq!(empty_tiles, linked_empty_tiles);
    }
}

// This is not a function on the struct because we need to use it while things are partially borrowed.
fn change_liberty_edges_at(
    size: u8,
    tiles: &mut [Content],
    groups: &mut [Group],
    tile: FlatTile,
    delta: i16,
    skip_group_id: Option<u16>,
) {
    for adj in tile.all_adjacent(size) {
        if let Some(group_id) = tiles[adj.index() as usize].group_id {
            if Some(group_id) != skip_group_id {
                let count = &mut groups[group_id as usize].liberty_edge_count;
                *count = count.wrapping_add_signed(delta);
            }
        }
    }
}

impl Group {
    fn is_dead(&self) -> bool {
        self.stones.is_empty()
    }
}

impl Eq for Chains {}

impl PartialEq for Chains {
    fn eq(&self, other: &Self) -> bool {
        // TODO see if this optimizes to something decent
        self.tiles.len() == other.tiles.len()
            && self.zobrist() == other.zobrist()
            && FlatTile::all(self.size).all(|tile| self.stone_at(tile) == other.stone_at(tile))
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
                let tile = Tile::new(x, y).to_flat(size);
                match self.tiles[tile.index() as usize].group_id {
                    None => write!(f, "   .")?,
                    Some(group) => write!(f, "{:4}", group)?,
                }
            }
            writeln!(f)?;
        }
        write!(f, "       ")?;
        for x in 0..size {
            write!(f, "   {}", TileX(x))?;
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

impl Linked for Content {
    fn link(&self) -> &LinkNode {
        &self.link
    }

    fn link_mut(&mut self) -> &mut LinkNode {
        &mut self.link
    }
}

impl Linked for Group {
    fn link(&self) -> &LinkNode {
        &self.dead_link
    }

    fn link_mut(&mut self) -> &mut LinkNode {
        &mut self.dead_link
    }
}
