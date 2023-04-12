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
    zobrist: Zobrist,
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
    pub zobrist_next: Zobrist,
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
        Chains {
            size,
            tiles: vec![Content::default(); size as usize * size as usize],
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

    // TODO rename to place_stone
    pub fn place_tile(&mut self, place_tile: Tile, place_stone: Player) -> Result<PlacementKind, TileOccupied> {
        let prepared = self.prepare_place_tile(place_tile, place_stone)?;
        let PreparedPlacement {
            kind,
            new_group,
            merge_friendly,
            clear_enemy,
        } = prepared;

        // TODO reintroduce fast case if
        // * at most one merged group
        // * no cleared enemy groups

        match kind {
            PlacementKind::Normal | PlacementKind::Capture => {
                let new_group_id = self.allocate_group(new_group);
                self.set_stone_at(place_tile, place_stone, new_group_id);
                self.update_tile_groups(&clear_enemy, place_stone.other(), &merge_friendly, new_group_id);
            }
            PlacementKind::SuicideSingle => {
                // don't do anything, we don't even need to place the stone
            }
            PlacementKind::SuicideMulti => {
                // we don't need to actually place the stone
                //   clear the merged friendly groups, don't merge anything
                self.update_tile_groups(&merge_friendly, place_stone, &[], u16::MAX);
            }
        }

        Ok(kind)
    }

    fn set_stone_at(&mut self, tile: Tile, stone: Player, group: u16) {
        let size = self.size();

        // update tile itself
        debug_assert!(self.stone_at(tile).is_none());
        self.tiles[tile.index(size)].group_id = Some(group);

        // update hash
        self.zobrist ^= Zobrist::for_player_tile(stone, tile, size);

        // decrease liberty of adjacent
        for adj in Tile::all_adjacent(tile, size) {
            if let Some(adj_group_id) = self.content_at(adj).group_id {
                self.groups[adj_group_id as usize].liberty_edge_count -= 1;
            }
        }
    }

    fn clear_stone_at(&mut self, tile: Tile, stone: Player, clear: &[u16], merge: &[u16], into: u16) {
        let size = self.size();

        // update tile itself
        debug_assert!(self.stone_at(tile) == Some(stone));
        self.tiles[tile.index(size)].group_id = None;

        // update hash
        self.zobrist ^= Zobrist::for_player_tile(stone, tile, size);

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
    fn update_tile_groups(&mut self, clear: &[u16], clear_player: Player, merge: &[u16], into: u16) {
        // update the tiles
        for tile in Tile::all(self.size()) {
            let size = self.size();

            let content = &mut self.tiles[tile.index(size)];
            if let Some(group_id) = content.group_id {
                if clear.contains(&group_id) {
                    self.clear_stone_at(tile, clear_player, clear, merge, into);
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

    // TODO rename to simulate_place_stone
    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub fn simulate_place_tile(
        &self,
        place_tile: Tile,
        place_stone: Player,
    ) -> Result<SimulatedPlacement, TileOccupied> {
        let prepared = self.prepare_place_tile(place_tile, place_stone)?;
        todo!("simulate_place_tile")
    }

    // TODO unroll this whole thing into the 4 directions?
    //    collected inputs: type/group_id/liberties for each size
    //    outputs: what groups to merge and what groups to kill
    //    => simple function with 4 inputs, 4 outputs
    pub fn prepare_place_tile(&self, place_tile: Tile, place_stone: Player) -> Result<PreparedPlacement, TileOccupied> {
        let size = self.size;
        let content = self.tiles[place_tile.index(size)];
        if content.group_id.is_some() {
            return Err(TileOccupied);
        }

        let all_adjacent = place_tile.all_adjacent(size);

        // investigate adjacent tiles
        let mut new_group = Group {
            player: place_stone,
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

                    if group.player == place_stone {
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
        assert_eq!(self.zobrist, new_zobrist, "Invalid zobrist hash");
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
