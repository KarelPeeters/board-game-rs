use internal_iterator::InternalIterator;
use itertools::Itertools;

use board_game::games::scrabble::basic::{Deck, Letter, Mask};
use board_game::games::scrabble::grid::{Cell, ScrabbleGrid};

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

// scrabble multiplier values
//  w: double word
//  W: triple word
//  l: double letter
//  L: triple letter
const MULTIPLIERS: &str = "
W..l...W...l..W
.w...L...L...w.
..w...l.l...w..
l..w...l...w..l
....w.....w....
.L...L...L...L.
..l...l.l...l..
W..l...w...l..W
..l...l.l...l..
.L...L...L...L.
....w.....w....
l..w...l...w..l
..w...l.l...w..
.w...L...L...w.
W..l...W...l..W
";

fn main() {
    // gen_fst();
    let set = load_fst();

    let empty_cell = Cell {
        letter: None,
        letter_multiplier: 1,
        word_multiplier: 1,
        allowed_by_dir: [Mask::NONE; 2],
        score_by_dir: [0; 2],
        attached: false,
    };
    let mut grid = ScrabbleGrid {
        width: 15,
        height: 15,
        cells: (0..15 * 15).map(|_| empty_cell.clone()).collect_vec(),
    };

    // set letters
    for (y, line) in GRID.trim().lines().enumerate() {
        for (x, c) in line.trim().chars().enumerate() {
            if c != '.' {
                // set letter
                let cell = grid.cell_mut(x as u8, y as u8);
                let letter = Letter::from_char(c).unwrap();
                cell.letter = Some(letter);

                // set neighbors attached
                if x > 0 {
                    grid.cell_mut(x as u8 - 1, y as u8).attached = true;
                }
                if x < 14 {
                    grid.cell_mut(x as u8 + 1, y as u8).attached = true;
                }

                if y > 0 {
                    grid.cell_mut(x as u8, y as u8 - 1).attached = true;
                }
                if y < 14 {
                    grid.cell_mut(x as u8, y as u8 + 1).attached = true;
                }
            }
        }
    }
    for (y, line) in MULTIPLIERS.trim().lines().enumerate() {
        for (x, c) in line.trim().chars().enumerate() {
            if c != '.' {
                // set letter
                let cell = grid.cell_mut(x as u8, y as u8);
                match c {
                    'w' => cell.word_multiplier = 2,
                    'W' => cell.word_multiplier = 3,
                    'l' => cell.letter_multiplier = 2,
                    'L' => cell.letter_multiplier = 3,
                    _ => panic!("invalid char {:?}", c),
                }
            }
        }
    }

    grid.update_all_allowed(&set);
    grid.assert_valid(&set);

    // for y in 0..grid.height {
    //     for x in 0..grid.width {
    //         println!("({}, {}) => {:?}", x, y, grid.cell(x, y));
    //     }
    // }

    // let deck = Deck::from_letters("DGILOPR").unwrap();
    // let deck = Deck::from_letters("GID").unwrap();
    let deck = Deck::from_letters("GLID").unwrap();

    grid.available_moves(&set, deck).for_each(|mv| {
        println!("{:?}", mv);
    });

    let mv = grid.available_moves(&set, deck).next().unwrap();

    println!("playing {:?}", mv);

    let new_deck = grid.play(&set, mv, deck);
    grid.assert_valid(&set);

    println!("deck after: {:?}", new_deck);
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
