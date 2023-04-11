#[cfg(test)]
mod test {
    use board_game::board::Player;
    use board_game::games::go::{Chains, Group, Rules, Tile};

    fn build_chains(size: u8, rules: Rules, tiles: &[((u8, u8), Player)]) -> Chains {
        let mut chains = Chains::new(size);
        for &((x, y), player) in tiles {
            chains.place_tile_full(Tile::new(x, y), player, &rules);
        }
        chains
    }

    #[test]
    fn corner_triangle_corner_first() {
        let tiles = [((0, 0), Player::A), ((0, 1), Player::A), ((1, 0), Player::A)];
        let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

        // TODO add asserts
        println!("{}", chains);
    }

    #[test]
    fn corner_triangle_corner_last() {
        let tiles = [((0, 1), Player::A), ((1, 0), Player::A), ((0, 0), Player::A)];
        let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

        // TODO add asserts
        println!("{}", chains);
    }

    #[test]
    fn merge_long_overlapping() {
        let mut tiles = vec![];
        for y in 0..5 {
            tiles.push(((1, y), Player::A));
            tiles.push(((3, y), Player::A));
        }
        tiles.push(((2, 0), Player::A));

        let chains = build_chains(5, Rules::tromp_taylor(), &tiles);

        println!("{}", chains);

        let expected = Group {
            player: Player::A,
            stone_count: 5 + 5 + 1,
            liberty_count: 5 + 5 + 4,
        };
        assert_eq!(chains.group(Tile::new(2, 0)), Some(expected));
    }
}
