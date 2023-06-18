use internal_iterator::InternalIterator;
use itertools::Itertools;

use board_game::games::scrabble::basic::{Deck, Letter, Mask};
use board_game::games::scrabble::grid::{Cell, ScrabbleGrid};

type Set = fst::Set<Vec<u8>>;

fn main() {
    let empty_cell = Cell {
        letter: None,
        letter_multiplier: 1,
        word_multiplier: 1,
        allowed_horizontal: Mask::ALL_LETTERS,
        allowed_vertical: Mask::ALL_LETTERS,
        attached_horizontal: false,
        attached_vertical: false,
    };
    let mut grid = ScrabbleGrid {
        width: 15,
        height: 15,
        cells: (0..15 * 15).map(|_| empty_cell.clone()).collect_vec(),
    };

    grid.cell_mut(7, 7).attached_horizontal = true;

    let set = load_fst();
    let deck = Deck::from_letters("TEST").unwrap();

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

            let combined = if prefix.is_empty() {
                suffix.to_string()
            } else {
                let prefix_rev = prefix.chars().rev().collect::<String>();
                format!("{}+{}", suffix, prefix_rev)
            };

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
