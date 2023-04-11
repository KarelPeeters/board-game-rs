use itertools::Itertools;

use board_game::board::Player;
use board_game::games::go::{Chains, Group, Rules, Tile};

fn build_chains(size: u8, rules: Rules, tiles: &[(u8, u8, Player)]) -> Chains {
    let mut chains = Chains::new(size);
    for &(x, y, player) in tiles {
        chains.place_tile_full(Tile::new(x, y), player, &rules);
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

    chains.place_tile_full(Tile::new(1, 0), Player::B, &rules);
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

    chains.place_tile_full(Tile::new(2, 2), Player::A, &rules);
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
