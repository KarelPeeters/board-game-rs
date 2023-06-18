use fst::raw::Node;
use itertools::Itertools;

use board_game::games::scrabble::{Deck, Letter, Mask};

type Set = fst::Set<Vec<u8>>;

fn main() {
    // gen_fst();

    let set = load_fst();

    // println!("FST items: {}", set.len());
    // println!("FST bytes: {}", set.as_fst().as_bytes().len());
    // println!("{}", set.contains("TSET+"));

    let cells = vec![
        Cell {
            letter: None,
            allowed_orth: Mask::ALL_LETTERS,
            has_neighbor: false,
        },
        Cell {
            letter: Some(Letter::from_char('E').unwrap()),
            allowed_orth: Mask::from_letters("E").unwrap(),
            has_neighbor: false,
        },
        Cell {
            letter: None,
            allowed_orth: Mask::ALL_LETTERS,
            has_neighbor: false,
        },
        Cell {
            letter: None,
            allowed_orth: Mask::ALL_LETTERS,
            has_neighbor: false,
        },
        Cell {
            letter: None,
            allowed_orth: Mask::ALL_LETTERS,
            has_neighbor: true,
        },
    ];

    let deck = Deck::from_letters("TEST").unwrap();

    movegen(&set, &cells, deck, |mv| println!("{:?}", mv));
}

#[derive(Debug)]
struct Move {
    anchor: usize,
    forward_count: usize,
    word: String,
}

struct Cell {
    letter: Option<Letter>,
    allowed_orth: Mask,
    has_neighbor: bool,
}

struct MoveGen<'a, F: FnMut(Move)> {
    set: &'a Set,
    cells: &'a [Cell],
    anchor: usize,
    max_back: usize,
    f: F,
}

fn movegen(set: &Set, cells: &[Cell], deck: Deck, mut f: impl FnMut(Move)) {
    let mut prev_anchor = None;

    for curr in 0..cells.len() {
        let cell = &cells[curr];
        let is_anchor = cell.letter.is_none() && cell.has_neighbor;
        if !is_anchor {
            continue;
        }

        let max_back = prev_anchor.map_or(curr, |prev| curr - prev - 1);
        prev_anchor = Some(curr);

        let mut movegen = MoveGen {
            set,
            cells,
            anchor: curr,
            max_back,
            f: &mut f,
        };
        movegen.run(deck);
    }
}

impl<F: FnMut(Move)> MoveGen<'_, F> {
    fn run(&mut self, deck: Deck) {
        println!(
            "starting movegen anchor={} max_back={} {:?}",
            self.anchor, self.max_back, deck
        );

        let root = self.set.as_fst().root();
        let mut curr = vec![];

        self.run_recurse_forward(deck, root, &mut curr);

        debug_assert!(curr.is_empty());
    }

    fn run_recurse_forward(&mut self, mut deck: Deck, node: Node, curr: &mut Vec<u8>) {
        println!("run_recurse_forward {:?} {:?}", deck, curr);

        self.maybe_report(node, curr.len(), curr);

        let cell = self.anchor + curr.len();
        if cell >= self.cells.len() {
            // hit right edge, we have to switch direction now
            if let Some(index) = node.find_input(b'+') {
                let trans = node.transition(index);
                let next = self.set.as_fst().node(trans.addr);
                self.run_recurse_backward(deck, next, curr, curr.len());
            }

            return;
        }

        for trans in node.transitions() {
            let char = trans.inp;
            let next = self.set.as_fst().node(trans.addr);

            if char == b'+' {
                // switch direction
                self.run_recurse_backward(deck, next, curr, curr.len());
            } else {
                // place letter if possible
                let letter = Letter::from_char(char as char).unwrap();

                if self.cells[cell].allowed_orth.get(letter) {
                    let used_deck = if let Some(actual) = self.cells[cell].letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    curr.push(char);
                    self.run_recurse_forward(deck, next, curr);
                    let popped = curr.pop();
                    debug_assert_eq!(popped, Some(char));

                    if used_deck {
                        deck.add(letter);
                    }
                }
            }
        }
    }

    fn run_recurse_backward(&mut self, mut deck: Deck, node: Node, curr: &mut Vec<u8>, forward_count: usize) {
        println!("run_recurse_back {:?} {:?} {}", deck, curr, forward_count);

        self.maybe_report(node, forward_count, curr);

        let back_count = curr.len() - forward_count;
        if back_count >= self.max_back {
            // we hit the left edge, stop
            return;
        }
        let cell = self.anchor - back_count;

        for trans in node.transitions() {
            let char = trans.inp;
            let next = self.set.as_fst().node(trans.addr);

            if char == b'+' {
                unreachable!()
            } else {
                let letter = Letter::from_char(char as char).unwrap();

                if self.cells[cell].allowed_orth.get(letter) {
                    let used_deck = if let Some(actual) = self.cells[cell].letter {
                        debug_assert_eq!(actual, letter);
                        false
                    } else if deck.try_remove(letter) {
                        true
                    } else {
                        continue;
                    };

                    curr.push(char);
                    self.run_recurse_backward(deck, next, curr, forward_count);
                    curr.pop();

                    if used_deck {
                        deck.add(letter);
                    }
                }
            }
        }
    }

    fn maybe_report(&mut self, node: Node, forward_count: usize, word: &[u8]) {
        if node.is_final() {
            let mv = Move {
                anchor: self.anchor,
                forward_count,
                word: String::from_utf8(word.to_vec()).unwrap(),
            };
            (self.f)(mv);
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
