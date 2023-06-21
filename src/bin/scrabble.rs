use std::sync::Arc;

use internal_iterator::InternalIterator;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::Rng;

use board_game::ai::solver::solve_all_moves;
use board_game::board::{Board, BoardMoves, Player};
use board_game::games::scrabble::basic::{Deck, Letter, MAX_DECK_SIZE};
use board_game::games::scrabble::board::ScrabbleBoard;
use board_game::games::scrabble::grid::ScrabbleGrid;
use board_game::games::scrabble::movegen::PlaceMove;
use board_game::util::tiny::consistent_rng;

type Set = fst::Set<Vec<u8>>;

const GRID: &str = "
.......L.......
.......A.J.....
.......TWINER..
...A...E.VOTER.
.BURAN.E.E.....
...CHUTNEYS....
...A...........
WOOD...S.......
E..IF..L.......
I..AA..E.......
R.C.U.PAM......
D.ALGUAZIL....C
O.N.H.VEX...F.Y
E.T..KI.TOONIES
S...MIDI....B.T
";

fn main() {
    // gen_fst();
    let set = Arc::new(load_fst());

    // fuzz(&set);
    // return;

    let mut grid = ScrabbleGrid::from_str_2d(&set, GRID.trim()).unwrap();
    grid.copy_multipliers_from(&ScrabbleGrid::default());
    grid.assert_valid(&set);

    let mut board = ScrabbleBoard {
        grid,
        next_player: Player::A,
        deck_a: Deck::from_letters("DGILOPR").unwrap(),
        deck_b: Deck::from_letters("EGNOQR").unwrap(),
        score_a: 420,
        score_b: 369,
        exchange_count: 0,
        set: set.clone(),
    };
    println!("{}", board);

    // board.available_moves().unwrap().for_each(|mv| {
    //     println!("{:?}", mv);
    // });
    // let mv = board.available_moves().unwrap().max_by_key(|mv| mv.score).unwrap();
    // println!("playing {:?}", mv);
    // board.play(mv).unwrap();
    // println!("{}", board);

    for depth in 0.. {
        println!("depth: {}", depth);
        let result = solve_all_moves(&board, depth);
        println!("  {:?}", result);
    }
}

fn fuzz(set: &Set) {
    let mut rng = consistent_rng();

    loop {
        let mut grid = ScrabbleGrid::default();
        println!("{}", grid);

        let mut fails = 0;

        loop {
            let count = rng.gen_range(1..=MAX_DECK_SIZE);
            let letters = (0..count)
                .map(|_| Letter::all().choose(&mut rng).unwrap().to_char())
                .collect::<String>();
            let deck = Deck::from_letters(&letters).unwrap();
            println!("Deck: {:?}", deck);

            let moves: Vec<PlaceMove> = grid.available_moves(set, deck).collect();

            let mv = match moves.choose(&mut rng) {
                None => {
                    fails += 1;
                    if fails < 100 {
                        continue;
                    } else {
                        break;
                    }
                }
                Some(&mv) => {
                    fails = 0;
                    mv
                }
            };

            println!("Playing {:?}", mv);
            let new_deck = grid.play(set, mv, deck).unwrap();
            println!("Deck after: {:?}", new_deck);

            println!("Grid after:");
            println!("{}", grid);

            grid.assert_valid(set);

            println!();
        }
    }
}

fn load_fst() -> Set {
    let bytes = std::fs::read("ignored/fst.bin").unwrap();
    Set::new(bytes).unwrap()
}

fn gen_fst() {
    let file = std::fs::read_to_string("ignored/Collins Scrabble Words (2019).txt").unwrap();

    println!("Collecting expanded strings");
    let mut expanded = vec![];
    let mut word_count = 0;

    for word in file.lines() {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }

        word_count += 1;

        for i in 0..word.len() {
            let prefix = &word[..i];
            let suffix = &word[i..];
            assert!(!suffix.is_empty());

            // having a trailing + for complete words is important to force a direction switch
            let prefix_rev = prefix.chars().rev().collect::<String>();
            let combined = format!("{}+{}", suffix, prefix_rev);

            expanded.push(combined);
        }
    }

    println!("Sorting list");
    expanded.sort_unstable();

    println!("collected {} lines -> {} expanded", word_count, expanded.len());

    println!("Building set");
    let set = Set::from_iter(expanded.iter()).unwrap();

    println!("map len: {}", set.len());
    println!("map bytes: {}", set.as_fst().as_bytes().len());

    std::fs::write("ignored/fst.bin", set.as_fst().as_bytes()).unwrap();
}
