use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::zip;

use crate::board::Player;
use crate::games::go::tile::{Direction, Tile};
use crate::games::go::{Rules, Score};

// TODO add function to remove stones?
//   could be tricky since groups would have to be split
//   can be pretty slow
#[derive(Debug, Clone, Eq)]
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
    pub liberty_count: u16,
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

    pub fn content(&self, tile: Tile) -> Content {
        self.tiles[tile.index(self.size)]
    }

    pub fn group(&self, tile: Tile) -> Option<Group> {
        self.content(tile).group_id.map(|id| self.groups[id as usize])
    }

    pub fn tile(&self, tile: Tile) -> Option<Player> {
        self.group(tile).map(|group| group.player)
    }

    /// Is there a path between `start` and another tile with value `target` over only `player` tiles?
    pub fn reaches(&self, start: Tile, target: Option<Player>) -> bool {
        // TODO implement more quickly with chains
        //   alternatively, keep this as a fallback for unit tests
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
                if let Some(adj) = tile.adjacent_in(dir, self.size) {
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

    pub fn score(&self) -> Score {
        // TODO rewrite using chains
        // TODO maybe even move to chains?

        let mut score_a = 0;
        let mut score_b = 0;

        for tile in Tile::all(self.size()) {
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

    // TODO store the current tile in the content too without the extra indirection?
    // TODO add fast path for exactly one friendly neighbor, just modify the existing group
    // TODO remove prints
    pub fn place_tile_full(&mut self, tile: Tile, curr: Player, rules: &Rules) -> bool {
        println!("placing tile {}, value {:?}", tile, curr);

        let content = self.tiles[tile.index(self.size)];
        assert!(content.group_id.is_none());

        let other = curr.other();
        let all_adjacent = tile.all_adjacent(self.size);

        // create a new pseudo group
        let initial_liberties = all_adjacent.clone().filter(|&adj| self.tile(adj).is_none()).count();
        let mut curr_group = Group {
            player: curr,
            stone_count: 1,
            liberty_count: initial_liberties as u16,
        };
        println!("  initial group: {:?}", curr_group);

        // merge with matching neighbors
        let mut merged_groups = vec![];
        for adj in all_adjacent.clone() {
            if let Some(group_id) = self.tiles[adj.index(self.size)].group_id {
                let other_group = &mut self.groups[group_id as usize];

                if other_group.player == curr {
                    println!("    merging with {}, id {}, {:?}", adj, group_id, other_group);
                    merged_groups.push(group_id);

                    curr_group.stone_count += other_group.stone_count;
                    // TODO this is wrong, we might be double counting liberties is both groups had overlapping ones
                    //   really? can't we just double count anyway but make sure to properly subtract twice as well?
                    curr_group.liberty_count += other_group.liberty_count - 1;

                    // mark other group as dead
                    other_group.stone_count = 0;
                }
            }
        }

        println!("  after merging: {:?}", curr_group);

        // push new group, reuse old id if possible
        // TODO speed up by keeping a free linked list of ids?
        // TODO only do all of this if there is no suicide
        let curr_group_id = match self.groups.iter().position(|g| g.stone_count == 0) {
            Some(id) => {
                println!("  reusing group id {}", id);
                self.groups[id] = curr_group;
                id as u16
            }
            None => {
                let id = self.groups.len() as u16;
                println!("  creating new group id {}", id);
                self.groups.push(curr_group);
                id
            }
        };

        // TODO replace with small size-4 on-stack vec
        let mut cleared_groups = vec![];

        // subtract liberty from enemies and clear if necessary
        let mut cleared_enemy = false;
        for adj in all_adjacent {
            if let Some(group_id) = self.tiles[adj.index(self.size)].group_id {
                let group = &mut self.groups[group_id as usize];
                if group.player == other {
                    println!("  subtracting liberty from enemy group {}", group_id);
                    group.liberty_count -= 1;
                    if group.liberty_count == 0 {
                        println!("  scheduling clearing of enemy group {}", group_id);
                        cleared_enemy |= true;
                        cleared_groups.push(group_id);
                    }
                }
            }
        }

        // check for suicide
        if !cleared_enemy && curr_group.liberty_count == 0 {
            assert!(
                curr_group.stone_count > 1,
                "1-stone suicide would immediately repeat so is never allowed"
            );
            assert!(
                rules.allow_multi_stone_suicide,
                "multi store suicide not allowed by current rules"
            );

            println!("  scheduling suicide of curr group {}", curr_group_id);
            cleared_groups.push(curr_group_id);
            true
        } else {
            false
        };

        // mark cleared groups as dead
        // TODO inline this with pushing them to vec?
        for &group in &cleared_groups {
            self.groups[group as usize].stone_count = 0;
        }

        // fixup per-tile-state
        for content in &mut self.tiles {
            if let Some(mut id) = content.group_id {
                // point merged groups to new id
                if merged_groups.contains(&id) {
                    content.group_id = Some(curr_group_id);
                    id = curr_group_id;
                }

                // remove dead stones
                // TODO can we just skip this? allow tiles to keep pointing to dead groups?
                if cleared_groups.contains(&id) {
                    content.group_id = None;
                }

                // TODO if adjacent to dead group add liberty to curr group
                //   careful about overlapping liberties again!
            }
        }

        // update content of the current tile
        // TODO if suicide don't do some of this
        // TODO should we update "has_had" in case of suicide? no, right?
        let content = &mut self.tiles[tile.index(self.size)];
        content.group_id = Some(curr_group_id);
        content.has_had_a |= curr == Player::A;
        content.has_had_b |= curr == Player::B;

        println!();

        !cleared_groups.is_empty()
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
                liberty_count: _,
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

impl Display for Chains {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chains {{")?;
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

        writeln!(f, "  groups:")?;
        for (i, group) in self.groups.iter().enumerate() {
            writeln!(f, "    group {}: {:?}", i, group)?;
        }

        writeln!(f)?;
        Ok(())
    }
}
