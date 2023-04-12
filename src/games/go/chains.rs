use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::board::Player;
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{Score, Zobrist};

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow
#[derive(Clone, Eq)]
pub struct Chains {
    size: u8,
    tiles: Vec<Content>,
    groups: Vec<Group>,
    zobrist_tiles: Zobrist,
}

// TODO compact into single u8
// TODO store the current tile in the content too without the extra indirection?
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Content {
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

    // TODO add hash to group so we can quickly remove the entire group
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SimulatedPlacement {
    pub zobrist_next: Zobrist,
    pub kind: PlacementKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlacementKind {
    Normal,
    Capture,
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
        Chains {
            size,
            tiles: vec![Content::default(); size as usize * size as usize],
            groups: vec![],
            zobrist_tiles: Zobrist::default(),
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

    pub fn zobrist(&self) -> Zobrist {
        self.zobrist_tiles
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

    fn set_tile(&mut self, tile: Tile, value: Player, group_id: u16) {
        let size = self.size();

        let content = &mut self.tiles[tile.index(size)];
        content.group_id = Some(group_id);
        self.zobrist_tiles ^= Zobrist::for_player_tile(value, tile, size);
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub fn simulate_place_tile(
        &self,
        place_tile: Tile,
        place_stone: Player,
    ) -> Result<SimulatedPlacement, TileOccupied> {
        todo!()
    }

    pub fn place_tile(&mut self, place_tile: Tile, place_stone: Player) -> Result<PlacementKind, TileOccupied> {
        let size = self.size;
        let content = self.tiles[place_tile.index(size)];
        if content.group_id.is_some() {
            return Err(TileOccupied);
        }

        // at point we commit to fully placing the tile, so we're allowed to mutate self
        //   we won't generate any more errors, ensuring self stays in a valid state

        // TODO enable fast path
        // if let Some(kind) = self.place_tile_fast(place_tile, place_stone) {
        //     return Ok(kind)
        // }

        let all_adjacent = place_tile.all_adjacent(size);

        // create a new pseudo group
        let initial_liberties = all_adjacent.clone().filter(|&adj| self.stone_at(adj).is_none()).count();
        let mut curr_group = Group {
            player: place_stone,
            stone_count: 1,
            liberty_edge_count: initial_liberties as u16,
        };

        // merge with matching neighbors
        let mut merged_groups = vec![];
        for adj in all_adjacent.clone() {
            if let Some(adj_group_id) = self.tiles[adj.index(size)].group_id {
                let adj_group = &mut self.groups[adj_group_id as usize];

                if adj_group.player == place_stone {
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
        // TODO move this to after suicide checks?
        let curr_group_id = self.allocate_group(curr_group);

        // TODO replace with small size-4 on-stack vec
        let mut cleared_groups = vec![];

        // subtract liberty from enemies and clear if necessary
        let mut cleared_enemy = false;
        for adj in all_adjacent {
            if let Some(group_id) = self.tiles[adj.index(size)].group_id {
                let group = &mut self.groups[group_id as usize];
                if group.player == place_stone.other() {
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
                // TODO fast path: just don't place the stone?
                //   don't forget to undo group allocation
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

        // place new tile, it's important that we do this before tile state fixing
        //   suicide stones will be removed there
        self.set_tile(place_tile, place_stone, curr_group_id);

        // fixup per-tile-state
        self.fix_tile_state(&merged_groups, curr_group_id, &cleared_groups);

        // construct result
        let kind = if cleared_enemy {
            PlacementKind::Capture
        } else if suicide {
            if curr_group.stone_count == 1 {
                PlacementKind::SuicideSingle
            } else {
                PlacementKind::SuicideMulti
            }
        } else {
            PlacementKind::Normal
        };
        Ok(kind)
    }

    /// Fast version of [place_tile_full] that only works in simple cases without any group merging or clearing.
    #[allow(dead_code)]
    fn place_tile_fast(&mut self, placed_tile: Tile, placed_value: Player) -> Option<PlacementKind> {
        let size = self.size();
        let all_adjacent = placed_tile.all_adjacent(size);

        // TODO also add single stone suicide fast path somewhere?

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

        if !matches {
            return None;
        }

        // remove liberties from adjacent groups
        for adj in all_adjacent {
            if let Some(id) = self.content_at(adj).group_id {
                self.groups[id as usize].liberty_edge_count -= 1;
            }
        }

        // get the right group for this tile
        let group_id = match friendly_group {
            None => {
                // create new group
                self.allocate_group(Group {
                    player: placed_value,
                    stone_count: 1,
                    liberty_edge_count: liberties,
                })
            }
            Some(group_id) => {
                // increment liberties of existing group
                let group = &mut self.groups[group_id as usize];
                group.stone_count += 1;
                group.liberty_edge_count += liberties;
                group_id
            }
        };

        // set current tile
        self.set_tile(placed_tile, placed_value, group_id);

        Some(PlacementKind::Normal)
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

                    // update hash
                    // TODO replace with per-group hash update?
                    let player = self.groups[id as usize].player;
                    self.zobrist_tiles ^= Zobrist::for_player_tile(player, tile, size);

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
        let mut used_groups = vec![];

        for tile in &self.tiles {
            if let Some(id) = tile.group_id {
                // group must must exist
                assert!((id as usize) < self.groups.len());
                let group = self.groups[id as usize];

                // group must be alive
                assert!(group.liberty_edge_count > 0 && group.stone_count > 0);

                used_groups.push(id);
            }
        }

        for (id, group) in self.groups.iter().enumerate() {
            // stone_count and liberty_edge_count must agree on whether the group is dead
            assert_eq!((group.stone_count == 0), (group.liberty_edge_count == 0));

            // groups must be used xor dead
            assert!(used_groups.contains(&(id as u16)) ^ group.is_dead());
        }

        // check hash validness
        let mut new_zobrist = Zobrist::default();
        for tile in Tile::all(self.size()) {
            if let Some(player) = self.stone_at(tile) {
                let value = Zobrist::for_player_tile(player, tile, self.size);
                new_zobrist ^= value;
            }
        }
        assert_eq!(self.zobrist_tiles, new_zobrist, "Invalid zobrist hash");
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

#[allow(clippy::derivable_impls)]
impl Default for Content {
    fn default() -> Self {
        Content { group_id: None }
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
