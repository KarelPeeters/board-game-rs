#[cfg(test)]
mod test {
    use itertools::Itertools;

    use board_game::board::Player;
    use board_game::games::go::{Chains, Group, Rules, Tile};

    fn build_chains(size: u8, rules: Rules, tiles: &[(u8, u8, Player)]) -> Chains {
        let mut chains = Chains::new(size);
        for &(x, y, player) in tiles {
            chains.place_tile_full(Tile::new(x, y), player, &rules);
        }
        chains
    }

    #[test]
    fn corner_triangle_corner_first() {
        let tiles = [(0, 0, Player::A), (0, 1, Player::A), (1, 0, Player::A)];
        let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

        // TODO add asserts
        println!("{}", chains);
    }

    #[test]
    fn corner_triangle_corner_last() {
        let tiles = [(0, 1, Player::A), (1, 0, Player::A), (0, 0, Player::A)];
        let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

        // TODO add asserts
        println!("{}", chains);
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
            stone_count: 5 + 5 + 1,
            liberty_count: 5 + 5 + 4,
        };
        assert_eq!(chains.group(Tile::new(2, 0)), Some(expected));
    }

    #[test]
    fn trap_cyclic_group() {
        let size = 5;
        let rules = Rules::tromp_taylor();

        let tiles = Tile::all(size)
            .filter_map(|tile| {
                if tile.x == 0 || tile.x == 4 || tile.y == 0 || tile.y == 4 {
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

        let expected_0 = Group {
            player: Player::A,
            stone_count: 16,
            liberty_count: 0,
        };
        let expected_1 = Group {
            player: Player::B,
            stone_count: 8,
            liberty_count: 1,
        };

        assert_eq!(chains.group(Tile::new(0, 0)), Some(expected_0));
        assert_eq!(chains.group(Tile::new(1, 1)), Some(expected_1));

        chains.place_tile_full(Tile::new(2, 2), Player::A, &rules);
        println!("{}", chains);
    }
}
