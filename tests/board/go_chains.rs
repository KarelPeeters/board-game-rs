use std::collections::HashMap;
use std::convert::TryInto;

use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{Rng, SeedableRng};

use board_game::board::Player;
use board_game::games::go::{Chains, Group, PlacementKind, Tile};

#[test]
fn corner_triangle_corner_first() {
    let tiles = [(0, 0, Player::A), (0, 1, Player::A), (1, 0, Player::A)];
    let chains = build_chains(5, &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = Group {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group_at(Tile::new(0, 0)), Some(expected));
}

#[test]
fn corner_triangle_corner_last() {
    let tiles = [(0, 1, Player::A), (1, 0, Player::A), (0, 0, Player::A)];
    let chains = build_chains(5, &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = Group {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group_at(Tile::new(0, 0)), Some(expected));
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

    let expected = Group {
        color: Player::A,
        stone_count: 11,
        liberty_edge_count: 19,
    };
    assert_eq!(chains.group_at(Tile::new(2, 0)), Some(expected));
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

    let expected = Group {
        color: Player::A,
        stone_count: 4,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group_at(Tile::new(0, 0)), Some(expected));
}

#[test]
fn capture_corner() {
    let mut chains = build_chains(5, &[(0, 0, Player::A), (0, 1, Player::B)]);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../b....");

    let kind = chains.place_stone(Tile::new(1, 0), Player::B).unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../.w...");
    assert_eq!(kind, PlacementKind::Capture);

    let expected = Group {
        color: Player::B,
        stone_count: 1,
        liberty_edge_count: 3,
    };
    assert_eq!(chains.group_at(Tile::new(1, 0)), Some(expected));
    assert_eq!(chains.group_at(Tile::new(0, 1)), Some(expected));

    chains_test_main(&chains);
}

#[test]
fn capture_cyclic_group() {
    let size = 5;

    let tiles = Tile::all(size)
        .filter_map(|tile| {
            let edge_x = tile.x == 0 || tile.x == 4;
            let edge_y = tile.y == 0 || tile.y == 4;
            if edge_x && edge_y {
                None
            } else if edge_x || edge_y {
                Some((tile.x, tile.y, Player::A))
            } else if tile != Tile::new(2, 2) {
                Some((tile.x, tile.y, Player::B))
            } else {
                None
            }
        })
        .collect_vec();

    let mut chains = build_chains(size, &tiles);
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./bwwwb/bw.wb/bwwwb/.bbb.");

    let expected_edge = Group {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 2,
    };
    let expected_core = Group {
        color: Player::B,
        stone_count: 8,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group_at(Tile::new(0, 2)), Some(expected_edge));
    assert_eq!(chains.group_at(Tile::new(4, 2)), Some(expected_edge));
    assert_eq!(chains.group_at(Tile::new(2, 0)), Some(expected_edge));
    assert_eq!(chains.group_at(Tile::new(2, 4)), Some(expected_edge));
    assert_eq!(chains.group_at(Tile::new(1, 1)), Some(expected_core));
    chains_test_main(&chains);

    let kind = chains.place_stone(Tile::new(2, 2), Player::A).unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./b...b/b.b.b/b...b/.bbb.");
    assert_eq!(kind, PlacementKind::Capture);

    let expected_edge_new = Group {
        color: Player::A,
        stone_count: 3,
        liberty_edge_count: 5,
    };
    let expected_center = Group {
        color: Player::A,
        stone_count: 1,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group_at(Tile::new(0, 2)), Some(expected_edge_new));
    assert_eq!(chains.group_at(Tile::new(4, 2)), Some(expected_edge_new));
    assert_eq!(chains.group_at(Tile::new(2, 0)), Some(expected_edge_new));
    assert_eq!(chains.group_at(Tile::new(2, 4)), Some(expected_edge_new));
    assert_eq!(chains.group_at(Tile::new(2, 2)), Some(expected_center));

    chains_test_main(&chains);
}

#[test]
fn fill_board() {
    let size = 5;

    let mut tiles = Tile::all(size).map(|t| (t.x, t.y, Player::A)).collect_vec();
    let last = tiles.pop().unwrap();
    let last_tile = Tile::new(last.0, last.1);

    let chains = build_chains(size, &tiles);
    println!("{}", chains);
    let expected = Group {
        color: Player::A,
        stone_count: size as u16 * size as u16 - 1,
        liberty_edge_count: 2,
    };
    assert_eq!(chains.group_at(Tile::new(0, 0)), Some(expected));

    {
        // ensure the full board gets suicide captured
        let mut new_chains = chains.clone();
        let kind = new_chains.place_stone(last_tile, Player::A).unwrap();
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), Chains::new(size).to_fen());
        assert_eq!(kind, PlacementKind::SuicideMulti);

        chains_test_main(&new_chains);
    }

    {
        // ensure the other player can capture the rest too
        let mut new_chains = chains.clone();
        let kind = new_chains.place_stone(last_tile, Player::B).unwrap();
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), "....w/...../...../...../.....");
        assert_eq!(kind, PlacementKind::Capture);

        chains_test_main(&new_chains);
    }
}

#[test]
fn capture_jagged() {
    let mut chains = Chains::from_fen("wbbb/wwbb/.bbw/wwww").unwrap();
    println!("{}", chains);

    let kind = chains.place_stone(Tile::new(0, 1), Player::B).unwrap();
    println!("{}", chains);
    assert_eq!(chains.to_fen(), "w.../ww../w..w/wwww");
    assert_eq!(kind, PlacementKind::Capture);

    let expected = Group {
        color: Player::B,
        stone_count: 9,
        liberty_edge_count: 9,
    };
    assert_eq!(chains.group_at(Tile::new(0, 0)), Some(expected));

    chains_test_main(&chains);
}

#[test]
#[ignore]
fn fuzz_test() {
    let sizes = 0..=19;
    let players = [Player::A, Player::B];

    let mut rng = SmallRng::seed_from_u64(0);

    for game_index in 0..1000 {
        let size = rng.gen_range(sizes.clone());
        let mut chains = Chains::new(size);
        println!("Starting game {} with size {}", game_index, size);

        // move limit
        for _move_index in 0..1000 {
            // pick random empty tile
            let tile = Tile::all(size)
                .filter(|&tile| chains.stone_at(tile).is_none())
                .choose(&mut rng);
            let tile = match tile {
                None => break,
                Some(tile) => tile,
            };

            // place stone on that tile
            let player = *players.choose(&mut rng).unwrap();
            chains.place_stone(tile, player).expect("Tile must be empty");
        }
    }
}

fn build_chains(size: u8, tiles: &[(u8, u8, Player)]) -> Chains {
    let mut chains = Chains::new(size);
    for &(x, y, player) in tiles {
        let tile = Tile::new(x, y);

        let simulated = chains.simulate_place_stone(tile, player).unwrap();

        println!("Placing {:?} {:?}", tile, player);
        let kind = chains.place_stone(tile, player).unwrap();
        println!("Kind: {:?}", kind);

        assert_eq!(kind, simulated.kind);
        assert_eq!(chains.zobrist(), simulated.zobrist_next);

        println!("Result:\n{}", chains);
        chains_test_main(&chains);
    }
    chains
}

pub fn chains_test_main(chains: &Chains) {
    chains.assert_valid();
    check_floodfill(chains);
    check_fen(chains);
}

fn check_fen(chains: &Chains) {
    let fen = chains.to_fen();
    let new = Chains::from_fen(&fen).unwrap();
    assert_eq!(chains.to_fen(), new.to_fen());
    for tile in Tile::all(chains.size()) {
        let group = chains.group_at(tile);
        let new_group = new.group_at(tile);
        assert_eq!(group, new_group, "Group mismatch at {:?}", tile);
    }
}

fn check_floodfill(chains: &Chains) {
    let size = chains.size();
    let floodfill = compute_floodfill(chains);

    assert_eq!(floodfill.groups.len(), chains.groups().count());

    let mut map_id = HashMap::new();

    for tile in Tile::all(size) {
        let index = tile.index(size);

        let expected_id = floodfill.tile_group[index];
        let expected_group = expected_id.map(|id| floodfill.groups[id]);

        let actual_id = chains.content_at(tile).group_id;
        let actual_group = chains.group_at(tile);

        assert_eq!(expected_group, actual_group, "Mismatched group at {:?}", tile);

        let prev = map_id.insert(expected_id, actual_id);
        if let Some(prev) = prev {
            assert_eq!(prev, actual_id, "Mismatched group id mapping at {:?}", tile);
        }
    }
}

#[derive(Debug)]
struct FloodFill {
    groups: Vec<Group>,
    tile_group: Vec<Option<usize>>,
}

fn compute_floodfill(chains: &Chains) -> FloodFill {
    let size = chains.size();
    let area = chains.area() as usize;

    let mut groups = vec![];
    let mut tile_group = vec![None; area];

    // figure out the group for each tile
    for start in Tile::all(size) {
        let start_index = start.index(size);
        if tile_group[start_index].is_some() {
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
            let curr_index = curr.index(size);

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
        groups.push(Group {
            color: player,
            stone_count: stone_count.try_into().unwrap(),
            liberty_edge_count: liberty_edge_count.try_into().unwrap(),
        });
    }

    // check that tiles are covered
    for tile in Tile::all(size) {
        if chains.stone_at(tile).is_some() {
            assert!(tile_group[tile.index(size)].is_some());
        }
    }

    FloodFill { groups, tile_group }
}
