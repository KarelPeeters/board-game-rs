use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::zip;

use crate::board::Player;
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{Rules, Score};

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow
#[derive(Clone, Eq)]
pub struct Chains {
    size: u8,
    tiles: Vec<Content>,
    groups: Vec<Group>,
}

// TODO compact into single u8
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Content {
    pub has_had_a: bool,
    pub has_had_b: bool,
    pub group_id: Option<u16>,
}

// TODO compact? we can at least force player into one of the other fields
// TODO do even even need player here if we also store the player in the tile itself?
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Group {
    pub player: Player,
    pub stone_count: u16,
    /// Similar to the liberty count, except that liberties may be counted multiple times.
    /// This is easier to compute incrementally but still enough to know if a group has `0` liberties left.
    pub liberty_edge_count: u16,
    // TODO also track the real liberty count?
    //   not necessary for correctness, just for heuristics and convenience
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidPlacement {
    // This tile was already occupied by a different stone.
    Occupied,
    // 1-stone suicide would immediately repeat so is never allowed.
    SuicideSingle,
    // Multi store suicide that is not allowed by current rules.
    SuicideMulti,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Placement {
    pub chains: Chains,

    /// suicide
    pub captured_self: bool,
    pub captured_other: bool,
    pub captured_any: bool,
}

impl Chains {
    pub const MAX_SIZE: u8 = 19;

    pub fn new(size: u8) -> Self {
        assert!(size <= Self::MAX_SIZE);
        Chains {
            size,
            tiles: vec![Content::default(); size as usize * size as usize],
            groups: vec![],
        }
    }

    pub fn size(&self) -> u8 {
        self.size
    }

    pub fn area(&self) -> u16 {
        self.size as u16 * self.size as u16
    }

    pub fn content_at(&self, tile: Tile) -> Content {
        self.tiles[tile.index(self.size)]
    }

    pub fn group_at(&self, tile: Tile) -> Option<Group> {
        self.content_at(tile).group_id.map(|id| self.groups[id as usize])
    }

    pub fn stone_at(&self, tile: Tile) -> Option<Player> {
        self.group_at(tile).map(|group| group.player)
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

    pub fn clear_history(&mut self) {
        // reset ownership history to only the stones that currently exist
        let groups = &self.groups;
        for content in &mut self.tiles {
            let curr_owner = content.group_id.map(|id| groups[id as usize].player);
            content.has_had_a = curr_owner == Some(Player::A);
            content.has_had_b = curr_owner == Some(Player::B);
        }
    }

    pub fn without_history(&self) -> Self {
        let mut result = self.clone();
        result.clear_history();
        result
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

    fn set_tile_group_and_hist(&mut self, tile: Tile, value: Player, group_id: u16, suicide: bool) {
        let size = self.size();

        let content = &mut self.tiles[tile.index(size)];
        content.group_id = Some(group_id);

        if !suicide {
            // TODO should we update "has_had" in case of suicide? no, right?
            content.has_had_a |= value == Player::A;
            content.has_had_b |= value == Player::B;
        }
    }

    // TODO store the current tile in the content too without the extra indirection?
    /// We take `self` by value to ensure it never gets left in an invalid state
    /// if we return an error and bail out halfway.
    pub fn place_tile(
        mut self,
        placed_tile: Tile,
        placed_value: Player,
        rules: &Rules,
    ) -> Result<Placement, InvalidPlacement> {
        let size = self.size;
        let content = self.tiles[placed_tile.index(size)];
        if content.group_id.is_some() {
            return Err(InvalidPlacement::Occupied);
        }

        // TODO enable fast path
        // match self.place_tile_fast(placed_tile, placed_value) {
        //     Ok(p) => return Ok(p),
        //     Err(chains) => self = chains,
        // }

        let all_adjacent = placed_tile.all_adjacent(size);

        // create a new pseudo group
        let initial_liberties = all_adjacent.clone().filter(|&adj| self.stone_at(adj).is_none()).count();
        let mut curr_group = Group {
            player: placed_value,
            stone_count: 1,
            liberty_edge_count: initial_liberties as u16,
        };

        // merge with matching neighbors
        let mut merged_groups = vec![];
        for adj in all_adjacent.clone() {
            if let Some(adj_group_id) = self.tiles[adj.index(size)].group_id {
                let adj_group = &mut self.groups[adj_group_id as usize];

                if adj_group.player == placed_value {
                    merged_groups.push(adj_group_id);

                    curr_group.stone_count += adj_group.stone_count;
                    curr_group.liberty_edge_count += adj_group.liberty_edge_count;
                    curr_group.liberty_edge_count -= 1;

                    // this also nicely handles the edge case where we merge the same group twice,
                    //  the second time both stone_count and liberty_edge_count will be zero
                    adj_group.mark_dead();
                }
            }
        }

        // push new group, reuse old id if possible
        // TODO speed up by keeping a free linked list of ids?
        // TODO only do all of this if there is no suicide
        // TODO try immediately reusing existing friendly group without even checking the list?
        let curr_group_id = self.allocate_group(curr_group);

        // TODO replace with small size-4 on-stack vec
        let mut cleared_groups = vec![];

        // subtract liberty from enemies and clear if necessary
        let mut cleared_enemy = false;
        for adj in all_adjacent {
            if let Some(group_id) = self.tiles[adj.index(size)].group_id {
                let group = &mut self.groups[group_id as usize];
                if group.player == placed_value.other() {
                    group.liberty_edge_count -= 1;
                    if group.liberty_edge_count == 0 {
                        cleared_enemy |= true;
                        cleared_groups.push(group_id);
                    }
                }
            }
        }

        // check for suicide
        let suicide = if !cleared_enemy && curr_group.liberty_edge_count == 0 {
            if curr_group.stone_count == 1 {
                return Err(InvalidPlacement::SuicideSingle);
            }
            if !rules.allow_multi_stone_suicide {
                return Err(InvalidPlacement::SuicideMulti);
            }

            cleared_groups.push(curr_group_id);
            true
        } else {
            false
        };

        // mark cleared groups as dead
        // TODO inline this with pushing them to vec?
        for &group in &cleared_groups {
            self.groups[group as usize].mark_dead();
        }

        // place new tile
        //   it's important that we do this before tile state fixing
        //   if suicide the tile-updating logic will remove it anyway
        self.set_tile_group_and_hist(placed_tile, placed_value, curr_group_id, suicide);

        // fixup per-tile-state
        self.fix_tile_state(&merged_groups, curr_group_id, &cleared_groups);

        Ok(Placement {
            chains: self,
            captured_self: suicide,
            captured_other: cleared_enemy,
            captured_any: suicide | cleared_enemy,
        })
    }

    /// Fast version of [place_tile_full] that only works in simple cases without any group merging or clearing.
    #[allow(dead_code)]
    fn place_tile_fast(self, placed_tile: Tile, placed_value: Player) -> Result<Placement, Self> {
        let size = self.size();
        let all_adjacent = placed_tile.all_adjacent(size);

        // check if the following conditions hold:
        // * at most one distinct friendly adjacent group
        // * both friendly and enemy adjacent groups should have enough liberties left
        let mut friendly_group = None;
        let mut enemy_count = 0;
        let mut matches = true;
        let mut liberties = 0;

        for adj in all_adjacent.clone() {
            let content = self.content_at(adj);
            match content.group_id {
                None => liberties += 1,
                Some(id) => {
                    let group = self.groups[id as usize];
                    if group.player == placed_value {
                        // TODO allow the same friendly group multiple times if there are enough liberties
                        // we already optimistically count new liberties
                        if friendly_group.is_some() || group.liberty_edge_count + liberties <= 1 {
                            matches = false;
                            break;
                        }
                        friendly_group = Some(id);
                    } else {
                        // this works: if we see the same enemy group multiple times, the Nth time we will
                        //   ensure it has at least N+1 liberties left
                        if group.liberty_edge_count <= enemy_count + 1 {
                            matches = false;
                            break;
                        }
                        enemy_count += 1;
                    }
                }
            }
        }

        matches &= liberties > 0;

        if matches {
            let mut result = self;

            // remove liberties from adjacent groups
            for adj in all_adjacent {
                if let Some(id) = result.content_at(adj).group_id {
                    result.groups[id as usize].liberty_edge_count -= 1;
                }
            }

            // get the right group for this tile
            let group_id = match friendly_group {
                None => {
                    // create new group
                    result.allocate_group(Group {
                        player: placed_value,
                        stone_count: 1,
                        liberty_edge_count: liberties,
                    })
                }
                Some(group_id) => {
                    // increment liberties of existing group
                    let group = &mut result.groups[group_id as usize];
                    group.stone_count += 1;
                    group.liberty_edge_count += liberties;
                    group_id
                }
            };

            // set current tile
            result.set_tile_group_and_hist(placed_tile, placed_value, group_id, false);

            Ok(Placement {
                chains: result,
                captured_self: false,
                captured_other: false,
                captured_any: false,
            })
        } else {
            // return self unmodified
            Err(self)
        }
    }

    #[inline(never)]
    fn fix_tile_state(&mut self, merged_groups: &[u16], curr_group_id: u16, cleared_groups: &[u16]) {
        let size = self.size();

        // TODO use flat iterator and Tile::from_index instead?
        for tile in Tile::all(size) {
            let content = &mut self.tiles[tile.index(size)];

            if let Some(mut id) = content.group_id {
                // point merged groups to new id
                if merged_groups.contains(&id) {
                    content.group_id = Some(curr_group_id);
                    id = curr_group_id;
                }

                // remove dead stones
                if cleared_groups.contains(&id) {
                    content.group_id = None;

                    // add liberties to adjacent groups
                    for adj in tile.all_adjacent(size) {
                        let adj_group_id_old = self.tiles[adj.index(size)].group_id;
                        let adj_group_id =
                            adj_group_id_old.map(|id| if merged_groups.contains(&id) { curr_group_id } else { id });

                        if let Some(adj_group_id) = adj_group_id {
                            let adj_group = &mut self.groups[adj_group_id as usize];
                            if !adj_group.is_dead() {
                                adj_group.liberty_edge_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn assert_valid(&self) {
        for tile in &self.tiles {
            if let Some(id) = tile.group_id {
                assert!((id as usize) < self.groups.len());
                let group = self.groups[id as usize];
                assert!(group.liberty_edge_count > 0 && group.stone_count > 0);

                if group.player == Player::A {
                    assert!(tile.has_had_a);
                }
                if group.player == Player::B {
                    assert!(tile.has_had_b);
                }
            }
        }

        for group in &self.groups {
            // stone_count and liberty_edge_count must agree on whether the group is dead
            assert_eq!((group.stone_count == 0), (group.liberty_edge_count == 0));
        }
    }

    fn tile_for_eq_hash(&self, content: Content) -> EqHashTile {
        let Content {
            has_had_a,
            has_had_b,
            group_id,
        } = content;

        let player = group_id.map(|id| {
            let Group {
                player,
                stone_count: _,
                liberty_edge_count: _,
            } = self.groups[id as usize];
            player
        });

        EqHashTile {
            has_had_a,
            has_had_b,
            player,
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

#[derive(Eq, PartialEq, Hash)]
struct EqHashTile {
    has_had_a: bool,
    has_had_b: bool,
    player: Option<Player>,
}

#[allow(clippy::derivable_impls)]
impl Default for Content {
    fn default() -> Self {
        Content {
            has_had_a: false,
            has_had_b: false,
            group_id: None,
        }
    }
}

impl PartialEq for Chains {
    fn eq(&self, other: &Self) -> bool {
        if self.tiles.len() != other.tiles.len() {
            return false;
        }
        zip(&self.tiles, &other.tiles).all(|(&self_content, &other_content)| {
            self.tile_for_eq_hash(self_content) == other.tile_for_eq_hash(other_content)
        })
    }
}

impl Hash for Chains {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // TODO switch to proper Zobrist hashing, this is pretty slow
        for content in &self.tiles {
            self.tile_for_eq_hash(*content).hash(state);
        }
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
