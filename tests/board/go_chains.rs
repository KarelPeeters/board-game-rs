use std::collections::HashMap;
use std::convert::TryInto;

use itertools::Itertools;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::Rng;

use board_game::board::Player;
use board_game::games::go::{Chains, Direction, FlatTile, Group, PlacementKind, Score, Tile};
use board_game::util::tiny::consistent_rng;

use crate::util::test_sampler_uniform;

#[test]
fn empty() {
    let chains = Chains::new(5);

    println!("{}", chains);
    chains_test_main(&chains);
}

#[test]
fn single() {
    let mut chains = Chains::new(5);
    chains
        .place_stone(Tile::new(0, 0).to_flat(chains.size()), Player::A)
        .unwrap();

    println!("{}", chains);
    chains_test_main(&chains);
}

#[test]
fn double_separate() {
    let mut chains = Chains::new(5);
    chains
        .place_stone(Tile::new(0, 0).to_flat(chains.size()), Player::A)
        .unwrap();
    chains
        .place_stone(Tile::new(2, 0).to_flat(chains.size()), Player::A)
        .unwrap();

    println!("{}", chains);
    chains_test_main(&chains);
}

#[test]
fn double_adjacent_same() {
    let mut chains = Chains::new(5);
    chains
        .place_stone(Tile::new(0, 0).to_flat(chains.size()), Player::A)
        .unwrap();

    println!("{}", chains);
    let placement = chains
        .simulate_place_stone(Tile::new(1, 0).to_flat(chains.size()), Player::A)
        .unwrap();
    println!("{:?}", placement);

    chains
        .place_stone(Tile::new(1, 0).to_flat(chains.size()), Player::A)
        .unwrap();

    println!("{}", chains);
    chains_test_main(&chains);
}

#[test]
fn double_adjacent_diff() {
    let mut chains = Chains::new(5);
    chains
        .place_stone(Tile::new(0, 0).to_flat(chains.size()), Player::A)
        .unwrap();
    chains
        .place_stone(Tile::new(1, 0).to_flat(chains.size()), Player::B)
        .unwrap();

    println!("{}", chains);
    chains_test_main(&chains);
}

#[test]
fn corner_triangle_corner_first() {
    let tiles = [(0, 0, Player::A), (0, 1, Player::A), (1, 0, Player::A)];
    let chains = build_chains(5, &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = GroupExpect {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    expected.assert_eq(chains.group_at(Tile::new(0, 0).to_flat(chains.size())));
}

#[test]
fn corner_triangle_corner_last() {
    let tiles = [(0, 1, Player::A), (1, 0, Player::A), (0, 0, Player::A)];
    let chains = build_chains(5, &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = GroupExpect {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    expected.assert_eq(chains.group_at(Tile::new(0, 0).to_flat(chains.size())));
}

#[test]
fn merge_long_overlapping() {
    let mut tiles = vec![];
    for y in 0..5 {
        tiles.push((1, y, Player::A));
        tiles.push((3, y, Player::A));
    }
    tiles.push((2, 0, Player::A));

    let chains = build_chains(5, &tiles);

    println!("{}", chains);

    let expected = GroupExpect {
        color: Player::A,
        stone_count: 11,
        liberty_edge_count: 19,
    };
    expected.assert_eq(chains.group_at(Tile::new(2, 0).to_flat(chains.size())));
}

#[test]
fn cyclic_group() {
    // test whether merging a group with itself works
    let tiles = [
        (0, 0, Player::A),
        (0, 1, Player::A),
        (1, 0, Player::A),
        (1, 1, Player::A),
    ];
    let chains = build_chains(5, &tiles);

    println!("{}", chains);

    let expected = GroupExpect {
        color: Player::A,
        stone_count: 4,
        liberty_edge_count: 4,
    };
    expected.assert_eq(chains.group_at(Tile::new(0, 0).to_flat(chains.size())));
}

#[test]
fn capture_corner() {
    let mut chains = build_chains(5, &[(0, 0, Player::A), (0, 1, Player::B)]);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../b....");

    let tile_dead = Tile::new(0, 0).to_flat(chains.size());
    let tile_final = Tile::new(1, 0).to_flat(chains.size());

    assert!(chains.has_had_stone_at(tile_dead, Player::A));
    assert!(!chains.has_had_stone_at(tile_dead, Player::B));
    assert!(!chains.has_had_stone_at(tile_final, Player::A));
    assert!(!chains.has_had_stone_at(tile_final, Player::B));

    let sim = chains
        .place_stone(Tile::new(1, 0).to_flat(chains.size()), Player::B)
        .unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../.w...");
    assert_eq!(sim.kind, PlacementKind::Capture);

    let expected = GroupExpect {
        color: Player::B,
        stone_count: 1,
        liberty_edge_count: 3,
    };
    expected.assert_eq(chains.group_at(Tile::new(1, 0).to_flat(chains.size())));
    expected.assert_eq(chains.group_at(Tile::new(0, 1).to_flat(chains.size())));

    assert!(chains.has_had_stone_at(tile_dead, Player::A));
    assert!(!chains.has_had_stone_at(tile_dead, Player::B));
    assert!(!chains.has_had_stone_at(tile_final, Player::A));
    assert!(chains.has_had_stone_at(tile_final, Player::B));

    chains_test_main(&chains);
}

#[test]
fn capture_cyclic_group() {
    let size = 5;

    let tiles = Tile::all(size)
        .filter_map(|tile| {
            let edge_x = tile.x() == 0 || tile.x() == 4;
            let edge_y = tile.y() == 0 || tile.y() == 4;
            if edge_x && edge_y {
                None
            } else if edge_x || edge_y {
                Some((tile.x(), tile.y(), Player::A))
            } else if tile != Tile::new(2, 2) {
                Some((tile.x(), tile.y(), Player::B))
            } else {
                None
            }
        })
        .collect_vec();

    let mut chains = build_chains(size, &tiles);
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./bwwwb/bw.wb/bwwwb/.bbb.");

    let expected_edge = GroupExpect {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 2,
    };
    let expected_core = GroupExpect {
        color: Player::B,
        stone_count: 8,
        liberty_edge_count: 4,
    };
    expected_edge.assert_eq(chains.group_at(Tile::new(0, 2).to_flat(chains.size())));
    expected_edge.assert_eq(chains.group_at(Tile::new(4, 2).to_flat(chains.size())));
    expected_edge.assert_eq(chains.group_at(Tile::new(2, 0).to_flat(chains.size())));
    expected_edge.assert_eq(chains.group_at(Tile::new(2, 4).to_flat(chains.size())));
    expected_core.assert_eq(chains.group_at(Tile::new(1, 1).to_flat(chains.size())));
    chains_test_main(&chains);

    let sim = chains
        .place_stone(Tile::new(2, 2).to_flat(chains.size()), Player::A)
        .unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./b...b/b.b.b/b...b/.bbb.");
    assert_eq!(sim.kind, PlacementKind::Capture);

    let expected_edge_new = GroupExpect {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 5,
    };
    let expected_center = GroupExpect {
        color: Player::A,
        stone_count: 1,
        liberty_edge_count: 4,
    };
    expected_edge_new.assert_eq(chains.group_at(Tile::new(0, 2).to_flat(chains.size())));
    expected_edge_new.assert_eq(chains.group_at(Tile::new(4, 2).to_flat(chains.size())));
    expected_edge_new.assert_eq(chains.group_at(Tile::new(2, 0).to_flat(chains.size())));
    expected_edge_new.assert_eq(chains.group_at(Tile::new(2, 4).to_flat(chains.size())));
    expected_center.assert_eq(chains.group_at(Tile::new(2, 2).to_flat(chains.size())));

    chains_test_main(&chains);
}

#[test]
fn fill_board() {
    let size = 5;

    let mut tiles = Tile::all(size).map(|t| (t.x(), t.y(), Player::A)).collect_vec();
    let last = tiles.pop().unwrap();
    let last_tile = Tile::new(last.0, last.1).to_flat(size);

    let chains = build_chains(size, &tiles);
    println!("{}", chains);
    let expected = GroupExpect {
        color: Player::A,
        stone_count: size as u16 * size as u16 - 1,
        liberty_edge_count: 2,
    };
    expected.assert_eq(chains.group_at(Tile::new(0, 0).to_flat(chains.size())));

    chains_test_main(&chains);

    {
        // ensure the full board gets suicide captured
        let mut new_chains = chains.clone();
        let sim = new_chains.place_stone(last_tile, Player::A).unwrap();
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), Chains::new(size).to_fen());
        assert_eq!(sim.kind, PlacementKind::SuicideMulti);

        chains_test_main(&new_chains);
    }

    {
        // ensure the other player can capture the rest too
        let mut new_chains = chains;
        let sim = new_chains.place_stone(last_tile, Player::B).unwrap();
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), "....w/...../...../...../.....");
        assert_eq!(sim.kind, PlacementKind::Capture);

        chains_test_main(&new_chains);
    }
}

#[test]
fn capture_jagged() {
    let mut chains = Chains::from_fen("wbbb/wwbb/.bbw/wwww").unwrap();
    println!("{}", chains);

    let sim = chains
        .place_stone(Tile::new(0, 1).to_flat(chains.size()), Player::B)
        .unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), "w.../ww../w..w/wwww");
    assert_eq!(sim.kind, PlacementKind::Capture);

    let expected = GroupExpect {
        color: Player::B,
        stone_count: 9,
        liberty_edge_count: 9,
    };
    expected.assert_eq(chains.group_at(Tile::new(0, 0).to_flat(chains.size())));

    chains_test_main(&chains);
}

#[test]
fn fill_board_simulation() {
    let chains = Chains::from_fen("bbbb./bbbbb/bbbbb/bbbbb/bbbbb").unwrap();
    println!("{}", chains);
    chains.assert_valid();

    let tile = Tile::new(4, 4).to_flat(chains.size());
    let color = Player::A;

    let sim = chains.simulate_place_stone(tile, color).unwrap();
    assert_eq!(sim.kind, PlacementKind::SuicideMulti);

    let mut real = chains;
    real.place_stone(tile, color).unwrap();
    real.assert_valid();

    assert_eq!(real.zobrist(), sim.next_zobrist);
    assert_eq!(real.stone_count(), sim.next_stone_count);
}

#[test]
#[ignore]
fn fuzz_test() {
    let sizes = 0..=19;
    let players = [Player::A, Player::B];

    let mut rng = consistent_rng();

    for game_index in 0..1000 {
        let size = rng.gen_range(sizes.clone());
        let mut chains = Chains::new(size);
        println!("Starting game {} with size {}", game_index, size);

        // move limit
        for _move_index in 0..1000 {
            // pick random empty tile
            let tile = FlatTile::all(size)
                .filter(|&tile| chains.stone_at(tile).is_none())
                .choose(&mut rng);
            let tile = match tile {
                None => break,
                Some(tile) => tile,
            };

            let player = *players.choose(&mut rng).unwrap();

            // check simulation
            let sim = chains.simulate_place_stone(tile, player).unwrap();

            // actually place stone
            let placed = chains.place_stone(tile, player).expect("Tile must be empty");

            // check simulation validness
            assert_eq!(placed, sim);
            assert_eq!(chains.zobrist(), sim.next_zobrist);
            assert_eq!(chains.stone_count(), sim.next_stone_count);
            if !sim.kind.is_suicide() {
                let actual_count = chains.group_at(tile).unwrap().stones.len();
                assert_eq!(actual_count, sim.new_group_stone_count);
            }

            // check validness
            //   unfortunately checking the sampling here is too slow
            chains_test_main_no_sample(&chains);
        }
    }
}

fn build_chains(size: u8, tiles: &[(u8, u8, Player)]) -> Chains {
    let mut chains = Chains::new(size);
    for &(x, y, player) in tiles {
        let tile = Tile::new(x, y).to_flat(chains.size());

        let simulated = chains.simulate_place_stone(tile, player).unwrap();

        println!("Placing {:?} {:?}", tile, player);
        let sim = chains.place_stone(tile, player).unwrap();
        println!("Kind: {:?}", sim.kind);

        assert_eq!(sim.kind, simulated.kind);
        assert_eq!(chains.zobrist(), simulated.next_zobrist);
        assert_eq!(chains.stone_count(), simulated.next_stone_count);

        println!("Result:\n{}", chains);
        chains_test_main(&chains);
    }
    chains
}

pub fn chains_test_main_no_sample(chains: &Chains) {
    chains.assert_valid();
    check_floodfill(chains);
    check_fen(chains);
    assert_eq!(compute_score(chains), chains.score());
}

pub fn chains_test_main(chains: &Chains) {
    chains_test_main_no_sample(chains);
    check_sample_uniform(chains);
}

fn check_fen(chains: &Chains) {
    let fen = chains.to_fen();
    let new = Chains::from_fen(&fen).unwrap();
    assert_eq!(chains.to_fen(), new.to_fen());

    for tile in FlatTile::all(chains.size()) {
        let group = chains.group_at(tile);
        let new_group = new.group_at(tile);

        match (group, new_group) {
            (None, None) => {}
            (Some(group), Some(new_group)) => {
                let &Group {
                    color,
                    ref stones,
                    liberty_edge_count,
                    zobrist,
                    dead_link: _,
                } = group;

                assert_eq!(color, new_group.color);
                assert_eq!(liberty_edge_count, new_group.liberty_edge_count);
                assert_eq!(zobrist, new_group.zobrist);

                let group_stones = stones.assert_valid_and_collect(chains.tiles());
                let new_group_stones = new_group.stones.assert_valid_and_collect(new.tiles());
                assert_eq!(group_stones, new_group_stones);
            }
            _ => panic!("Occupation does not match"),
        }
    }
}

fn check_floodfill(chains: &Chains) {
    let size = chains.size();
    let floodfill = compute_floodfill(chains);

    assert_eq!(floodfill.groups.len(), chains.groups().count());

    let mut map_id = HashMap::new();

    for tile in FlatTile::all(size) {
        let expected_id = floodfill.tile_group[tile.index() as usize];
        let expected_group = expected_id.map(|id| floodfill.groups[id]);

        let actual_id = chains.content_at(tile).group_id;
        let actual_group = chains.group_at(tile);

        match expected_group {
            None => assert!(actual_group.is_none()),
            Some(expected_group) => expected_group.assert_eq(actual_group),
        }

        let prev = map_id.insert(expected_id, actual_id);
        if let Some(prev) = prev {
            assert_eq!(prev, actual_id, "Mismatched group id mapping at {:?}", tile);
        }
    }
}

pub fn chains_test_simulate(chains: &Chains) {
    for tile in FlatTile::all(chains.size()) {
        for color in [Player::A, Player::B] {
            let sim = chains.simulate_place_stone(tile, color);
            let mut real = chains.clone();
            let result = real.place_stone(tile, color);

            match (sim, result) {
                (Err(e_sim), Err(e_result)) => assert_eq!(e_sim, e_result),
                (Ok(sim), Ok(placed)) => {
                    assert_eq!(sim, placed);
                    assert_eq!(real.zobrist(), sim.next_zobrist);
                    assert_eq!(real.stone_count(), sim.next_stone_count);
                }
                _ => panic!("Mismatched simulation result at {:?} {:?}", tile, color),
            }
        }
    }
}

fn check_sample_uniform(chains: &Chains) {
    let size = chains.size();
    let empty_tiles: Vec<(Tile, FlatTile)> = chains.empty_tiles().map(|t| (t.to_tile(size), t)).collect();

    let mut rng = consistent_rng();
    test_sampler_uniform(&empty_tiles, false, || {
        chains
            .random_empty_tile(&mut rng)
            .map(|tile_flat| (tile_flat.to_tile(size), tile_flat))
    });
}

#[derive(Debug)]
struct FloodFill {
    groups: Vec<GroupExpect>,
    tile_group: Vec<Option<usize>>,
}

fn compute_floodfill(chains: &Chains) -> FloodFill {
    let size = chains.size();
    let area = chains.area() as usize;

    let mut groups = vec![];
    let mut tile_group = vec![None; area];

    // figure out the group for each tile
    for start in FlatTile::all(size) {
        if tile_group[start.index() as usize].is_some() {
            // already part of another group
            continue;
        }
        let player = match chains.stone_at(start) {
            // empty tile
            None => continue,
            Some(curr) => curr,
        };

        // start floodfill from tile through curr, counting stones and liberty edges
        let new_group_id = groups.len();

        let mut todo = vec![start];
        let mut visited = vec![false; area];

        let mut stone_count: u64 = 0;
        let mut liberty_edge_count: u64 = 0;
        let mut liberties: u64 = 0;

        while let Some(curr) = todo.pop() {
            let curr_index = curr.index() as usize;

            match chains.stone_at(curr) {
                None => {
                    liberty_edge_count += 1;
                    if !visited[curr_index] {
                        liberties += 1;
                    }
                }
                Some(p) if p == player => {
                    if !visited[curr_index] {
                        stone_count += 1;
                        tile_group[curr_index] = Some(new_group_id);
                        todo.extend(curr.all_adjacent(size));
                    }
                }
                Some(_) => {}
            }

            visited[curr_index] = true;
        }

        let _ = liberties;
        groups.push(GroupExpect {
            color: player,
            stone_count: stone_count.try_into().unwrap(),
            liberty_edge_count: liberty_edge_count.try_into().unwrap(),
        });
    }

    // check that tiles are covered
    for tile in FlatTile::all(size) {
        if chains.stone_at(tile).is_some() {
            assert!(tile_group[tile.index() as usize].is_some());
        }
    }

    FloodFill { groups, tile_group }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
struct GroupExpect {
    color: Player,
    stone_count: u16,
    liberty_edge_count: u16,
}

impl GroupExpect {
    /// Assert the given group matches the expected values.
    fn assert_eq(self, actual: Option<&Group>) {
        let actual = actual.map(|actual| {
            let &Group {
                color,
                ref stones,
                liberty_edge_count,
                zobrist: _,
                dead_link: _,
            } = actual;

            GroupExpect {
                color,
                stone_count: stones.len(),
                liberty_edge_count,
            }
        });

        assert_eq!(Some(self), actual);
    }
}

fn compute_score(chains: &Chains) -> Score {
    let mut score_a = 0;
    let mut score_b = 0;

    for tile in FlatTile::all(chains.size()) {
        match chains.stone_at(tile) {
            None => {
                let reaches_a = reaches(chains, tile, Some(Player::A));
                let reaches_b = reaches(chains, tile, Some(Player::B));
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
pub fn reaches(chains: &Chains, start: FlatTile, target: Option<Player>) -> bool {
    let through = chains.stone_at(start);
    assert_ne!(through, target);

    let mut visited = vec![false; chains.area() as usize];
    let mut stack = vec![start];

    while let Some(tile) = stack.pop() {
        let index = tile.index() as usize;
        if visited[index] {
            continue;
        }
        visited[index] = true;

        for dir in Direction::ALL {
            if let Some(adj) = tile.adjacent_in(dir, chains.size()) {
                let value = chains.stone_at(adj);
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
