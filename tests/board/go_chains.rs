use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{Rng, SeedableRng};

use board_game::board::Player;
use board_game::games::go::{Chains, Group, Rules, Tile};

fn build_chains(size: u8, rules: Rules, tiles: &[(u8, u8, Player)]) -> Chains {
    let mut chains = Chains::new(size);
    for &(x, y, player) in tiles {
        chains = chains.place_tile_full(Tile::new(x, y), player, &rules).unwrap().chains;
        // TODO remove print
        println!("{}", chains);
        check_chains_valid(&chains, &rules);
    }
    chains
}

fn check_chains_valid(chains: &Chains, rules: &Rules) {
    // TODO try reconstructing the board in different orders?
    // TODO try fully computing liberties from scratch using floodfill

    let fen = chains.to_fen();
    let new = Chains::from_fen(&fen, rules).unwrap();

    assert_eq!(chains.to_fen(), new.to_fen());

    for tile in Tile::all(chains.size()) {
        let group = chains.group(tile);
        let new_group = new.group(tile);
        assert_eq!(group, new_group, "Group mismatch at {:?}", tile);
    }
}

#[test]
fn corner_triangle_corner_first() {
    let tiles = [(0, 0, Player::A), (0, 1, Player::A), (1, 0, Player::A)];
    let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = Group {
        player: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group(Tile::new(0, 0)), Some(expected));
}

#[test]
fn corner_triangle_corner_last() {
    let tiles = [(0, 1, Player::A), (1, 0, Player::A), (0, 0, Player::A)];
    let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../b..../bb...");

    let expected = Group {
        player: Player::A,
        stone_count: 3,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group(Tile::new(0, 0)), Some(expected));
}

#[test]
fn merge_long_overlapping() {
    let mut tiles = vec![];
    for y in 0..5 {
        tiles.push((1, y, Player::A));
        tiles.push((3, y, Player::A));
    }
    tiles.push((2, 0, Player::A));

    let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

    println!("{}", chains);

    let expected = Group {
        player: Player::A,
        stone_count: 11,
        liberty_edge_count: 19,
    };
    assert_eq!(chains.group(Tile::new(2, 0)), Some(expected));
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
    let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

    println!("{}", chains);

    let expected = Group {
        player: Player::A,
        stone_count: 4,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group(Tile::new(0, 0)), Some(expected));
}

#[test]
fn capture_corner() {
    let rules = Rules::tromp_taylor();
    let mut chains = build_chains(5, rules, &[(0, 0, Player::A), (0, 1, Player::B)]);

    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../b....");

    chains = chains
        .place_tile_full(Tile::new(1, 0), Player::B, &rules)
        .unwrap()
        .chains;
    println!("{}", chains);
    assert_eq!(chains.to_fen(), "...../...../...../w..../.w...");

    let expected = Group {
        player: Player::B,
        stone_count: 1,
        liberty_edge_count: 3,
    };
    assert_eq!(chains.group(Tile::new(1, 0)), Some(expected));
    assert_eq!(chains.group(Tile::new(0, 1)), Some(expected));

    check_chains_valid(&chains, &rules);
}

#[test]
fn capture_cyclic_group() {
    let size = 5;
    let rules = Rules::tromp_taylor();

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

    let mut chains = build_chains(size, rules, &tiles);
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./bwwwb/bw.wb/bwwwb/.bbb.");

    let expected_edge = Group {
        player: Player::A,
        stone_count: 3,
        liberty_edge_count: 2,
    };
    let expected_core = Group {
        player: Player::B,
        stone_count: 8,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group(Tile::new(0, 2)), Some(expected_edge));
    assert_eq!(chains.group(Tile::new(4, 2)), Some(expected_edge));
    assert_eq!(chains.group(Tile::new(2, 0)), Some(expected_edge));
    assert_eq!(chains.group(Tile::new(2, 4)), Some(expected_edge));
    assert_eq!(chains.group(Tile::new(1, 1)), Some(expected_core));

    chains = chains
        .place_tile_full(Tile::new(2, 2), Player::A, &rules)
        .unwrap()
        .chains;
    println!("{}", chains);
    assert_eq!(chains.to_fen(), ".bbb./b...b/b.b.b/b...b/.bbb.");

    let expected_edge_new = Group {
        player: Player::A,
        stone_count: 3,
        liberty_edge_count: 5,
    };
    let expected_center = Group {
        player: Player::A,
        stone_count: 1,
        liberty_edge_count: 4,
    };
    assert_eq!(chains.group(Tile::new(0, 2)), Some(expected_edge_new));
    assert_eq!(chains.group(Tile::new(4, 2)), Some(expected_edge_new));
    assert_eq!(chains.group(Tile::new(2, 0)), Some(expected_edge_new));
    assert_eq!(chains.group(Tile::new(2, 4)), Some(expected_edge_new));
    assert_eq!(chains.group(Tile::new(2, 2)), Some(expected_center));

    check_chains_valid(&chains, &rules);
}

#[test]
fn fill_board() {
    let size = 5;
    let rules = Rules::tromp_taylor();

    let mut tiles = Tile::all(size).map(|t| (t.x, t.y, Player::A)).collect_vec();
    let last = tiles.pop().unwrap();
    let last_tile = Tile::new(last.0, last.1);

    let chains = build_chains(size, rules, &tiles);
    println!("{}", chains);
    let expected = Group {
        player: Player::A,
        stone_count: size as u16 * size as u16 - 1,
        liberty_edge_count: 2,
    };
    assert_eq!(chains.group(Tile::new(0, 0)), Some(expected));

    {
        // ensure the full board gets suicide captured
        let new_chains = chains
            .clone()
            .place_tile_full(last_tile, Player::A, &rules)
            .unwrap()
            .chains;
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), Chains::new(size).to_fen());

        check_chains_valid(&new_chains, &rules);
    }

    {
        // ensure the other player can capture the rest too
        let new_chains = chains.place_tile_full(last_tile, Player::B, &rules).unwrap().chains;
        println!("{}", new_chains);
        assert_eq!(new_chains.to_fen(), "....w/...../...../...../.....");

        check_chains_valid(&new_chains, &rules);
    }
}

#[test]
fn capture_jagged() {
    let rules = Rules::tromp_taylor();
    let chains = Chains::from_fen("wbbb/wwbb/.bbw/wwww", &rules).unwrap();
    println!("{}", chains);

    let new_chains = chains
        .place_tile_full(Tile::new(0, 1), Player::B, &rules)
        .unwrap()
        .chains;
    println!("{}", new_chains);

    assert_eq!(new_chains.to_fen(), "w.../ww../w..w/wwww");

    let expected = Group {
        player: Player::B,
        stone_count: 9,
        liberty_edge_count: 9,
    };
    assert_eq!(new_chains.group(Tile::new(0, 0)), Some(expected));

    check_chains_valid(&new_chains, &rules);
}

#[test]
#[ignore]
fn fuzz_test() {
    let sizes = 0..=9;
    let players = [Player::A, Player::B];
    let rules = [Rules::tromp_taylor(), Rules::cgos()];

    let mut rng = SmallRng::seed_from_u64(0);

    for game_index in 0..1000 {
        let size = rng.gen_range(sizes.clone());
        let rules = rules.choose(&mut rng).unwrap();

        println!("Starting game {} with size {} and rules {:?}", game_index, size, rules);

        let mut chains = Chains::new(size);

        // move limit
        for move_index in 0..1000 {
            println!("  starting move {}", move_index);
            let prev_chains = chains.clone();

            // invalid move limit
            'tries: for _ in 0..10 {
                // pick random empty tile
                let tile = Tile::all(size)
                    .filter(|&tile| chains.tile(tile).is_none())
                    .choose(&mut rng);
                let tile = match tile {
                    None => break,
                    Some(tile) => tile,
                };

                // try playing on that tile
                let player = *players.choose(&mut rng).unwrap();

                let r = chains.place_tile_full(tile, player, rules);

                match r {
                    Ok(p) => {
                        chains = p.chains;
                        println!("success:");
                        println!("{}", chains);

                        check_chains_valid(&chains, &rules);

                        break 'tries;
                    }
                    Err(_) => {
                        println!("failed, restoring previous chains");
                        // restore previous chains
                        chains = prev_chains.clone()
                    }
                }
            }
        }
    }
}
