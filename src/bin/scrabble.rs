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

fn main() {
    // gen_fst();

    let empty_cell = Cell {
        letter: None,
        letter_multiplier: 1,
        word_multiplier: 1,
        allowed_horizontal: Mask::ALL_LETTERS,
        allowed_vertical: Mask::ALL_LETTERS,
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
                cell.allowed_horizontal.clear();
                cell.allowed_horizontal.set(letter, true);
                cell.allowed_vertical.clear();
                cell.allowed_vertical.set(letter, true);

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

    // TODO set allowed masks

    println!("{:?}", grid.cell(10, 0));

    let set = load_fst();
    // let deck = Deck::from_letters("DGILOPR").unwrap();
    // let deck = Deck::from_letters("GID").unwrap();
    let deck = Deck::from_letters("GLID").unwrap();

    grid.available_moves(&set, deck).for_each(|mv| {
        println!("{:?}", mv);
    });
}

fn load_fst() -> Set {
    let bytes = std::fs::read("ignored/fst.bin").unwrap();
    Set::new(bytes).unwrap()
}

fn gen_fst() {
    let file = std::fs::read_to_string("ignored/Collins Scrabble Words (2019).txt").unwrap();

    println!("Collecting expanded strings");
    let mut expanded = vec![];
    let mut line_count = 0;
    for line in file.lines() {
        if line.is_empty() {
            continue;
        }

        line_count += 1;

        for i in 0..line.len() {
            let prefix = &line[..i];
            let suffix = &line[i..];

            // having a trailing + for complete words is important to force a direction switch
            let prefix_rev = prefix.chars().rev().collect::<String>();
            let combined = format!("{}+{}", suffix, prefix_rev);

            expanded.push(combined)
        }
    }

    println!("collected {} lines -> {} expanded", line_count, expanded.len());

    println!("Sorting");
    expanded.sort();

    println!("Building set");
    let set = Set::from_iter(expanded).unwrap();

    println!("{}", set.len());
    println!("{}", set.contains("YRREB"));

    std::fs::write("ignored/fst.bin", set.as_fst().as_bytes()).unwrap();
}
