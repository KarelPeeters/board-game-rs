#![allow(dead_code)]

use std::cmp::{max, min, Reverse};
use std::collections::HashSet;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use internal_iterator::InternalIterator;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::Rng;

use board_game::ai::solver::solve_all_moves;
use board_game::board::{Board, BoardMoves, Player};
use board_game::games::scrabble::basic::{Deck, Letter, MAX_DECK_SIZE};
use board_game::games::scrabble::board::{Move, ScrabbleBoard};
use board_game::games::scrabble::grid::ScrabbleGrid;
use board_game::games::scrabble::movegen::PlaceMove;
use board_game::games::scrabble::zobrist::Zobrist;
use board_game::pov::{NonPov, PlayerBox};
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

    test(&set);

    // summarize_nodes(&set);
    // bench(set);
    // fuzz(&set);
    // derp(&set);

    solve(&set);
}

#[derive(Debug, Copy, Clone)]
struct TTEntry {
    hash: Zobrist,
    kind: TTEntryKind,
    value: i64,
    depth: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TTEntryKind {
    Exact,
    Lower,
    Upper,
    Empty,
}

fn solve(set: &Arc<Set>) {
    let tt_size = 16 * 1024 * 1024;

    let board = example_board(set);
    let empty_entry = TTEntry {
        hash: Default::default(),
        kind: TTEntryKind::Empty,
        value: 0,
        depth: 0,
    };

    for depth in 0.. {
        let mut tt = vec![empty_entry; tt_size];

        let value = negamax(&mut tt, &board, depth, -1, 1);
        println!("depth {}: {}", depth, value);
    }
}

fn board_value_pov(board: &ScrabbleBoard) -> i64 {
    let scores = board.scores().pov(board.next_player());
    scores.pov as i64 - scores.other as i64
}

// TODO unmake move?
fn negamax(tt: &mut Vec<TTEntry>, board: &ScrabbleBoard, depth: u32, mut a: i64, mut b: i64) -> i64 {
    // TODO do this after the is_done check?
    let a_orig = a;

    // leaf eval
    let board_eval = board_value_pov(board);
    if depth == 0 || board.is_done() {
        return board_eval;
    }

    // tt lookup
    let hash = board.zobrist_pov_without_score();
    let tt_index = hash.inner() as usize % tt.len();
    let entry = &tt[tt_index];

    if entry.kind != TTEntryKind::Empty && entry.hash == hash && entry.depth >= depth {
        let entry_value = board_eval + entry.value;

        match entry.kind {
            TTEntryKind::Exact => return entry_value,
            TTEntryKind::Lower => {
                a = max(a, entry_value);
            }
            TTEntryKind::Upper => {
                b = min(b, entry_value);
            }
            TTEntryKind::Empty => unreachable!(),
        }

        if a >= b {
            return entry.value;
        }
    }

    // iterate over children
    let mut value = i64::MIN;

    let mut moves: Vec<_> = board.available_moves().unwrap().collect();
    moves.sort_unstable_by_key(|mv| match mv {
        Move::Place(mv) => Reverse(mv.score),
        Move::Exchange => Reverse(0),
    });

    for mv in moves {
        let next = board.clone_and_play(mv).unwrap();
        let next_value = -negamax(tt, &next, depth - 1, -b, -a);
        value = max(value, next_value);
        a = max(a, next_value);
        if a >= b {
            break;
        }
    }

    // tt insert
    let kind = if value <= a_orig {
        TTEntryKind::Upper
    } else if value >= b {
        TTEntryKind::Lower
    } else {
        TTEntryKind::Exact
    };
    let entry = TTEntry {
        hash,
        kind,
        value: value - board_eval,
        depth,
    };
    tt[tt_index] = entry;

    // final return
    value
}

fn test(set: &Arc<Set>) {
    let board = example_board(set);

    let moves: Vec<_> = board.available_moves().unwrap().collect();

    let place_moves = moves
        .iter()
        .filter_map(|mv| match mv {
            Move::Place(mv) => Some(mv),
            Move::Exchange => None,
        })
        .collect_vec();

    let place_moves_unique = place_moves
        .iter()
        .unique_by(|mv| (mv.dir, mv.x, mv.y, mv.placed))
        .collect_vec();
    assert_eq!(place_moves.len(), place_moves_unique.len());

    println!("{} moves", moves.len());

    let single_count = moves
        .iter()
        .filter(|mv| match mv {
            Move::Place(mv) => mv.placed.len() == 1,
            Move::Exchange => false,
        })
        .count();

    println!("{} single", single_count);

    assert_eq!(moves.len(), 557);
    assert_eq!(single_count, 113);
}

fn example_board(set: &Arc<Set>) -> ScrabbleBoard {
    let mut grid = ScrabbleGrid::from_str_2d(set, GRID.trim()).unwrap();
    grid.copy_multipliers_from(&ScrabbleGrid::default());
    grid.assert_valid(&set);

    let board = ScrabbleBoard::new(
        grid,
        Player::A,
        PlayerBox::new(
            Deck::from_letters("DGILOPR").unwrap(),
            Deck::from_letters("EGNOQR").unwrap(),
        ),
        PlayerBox::new(369, 420),
        0,
        set.clone(),
    );
    board
}

fn derp(set: &Arc<Set>) {
    let mut grid = ScrabbleGrid::from_str_2d(&set, GRID.trim()).unwrap();
    grid.copy_multipliers_from(&ScrabbleGrid::default());
    grid.assert_valid(&set);

    let board = example_board(set);
    println!("{}", board);

    bench_single(&board);

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

fn bench_single(board: &ScrabbleBoard) -> u64 {
    let mut total: u64 = 0;

    let mut start = Instant::now();
    let mut count = 0;

    loop {
        total += board.available_moves().unwrap().count() as u64;
        black_box(total);

        count += 1;
        if count >= 10_000 {
            let tp = start.elapsed().as_nanos() as f64 / total as f64;
            println!("{} movegen/s", tp);
            count = 0;
            start = Instant::now();
        }
    }
}

fn fuzz(set: &Set) {
    let mut rng = consistent_rng();

    loop {
        let mut grid = ScrabbleGrid::default();
        println!("{}", grid);

        let mut fails = 0;

        loop {
            let deck_size = rng.gen_range(1..=MAX_DECK_SIZE);
            let deck = rand_deck(deck_size, &mut rng);
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

            let sim = grid.simulate_play(mv, deck).unwrap();
            let new_deck = grid.play(set, mv, deck).unwrap();

            println!("Deck after: {:?}", new_deck);

            println!("Grid after:");
            println!("{}", grid);

            assert_eq!(sim.deck, new_deck);
            assert_eq!(sim.zobrist, grid.zobrist());

            grid.assert_valid(set);

            println!();
        }
    }
}

fn bench(set: Arc<Set>) {
    let mut mv_count = 0;
    let mut start = Instant::now();
    let mut rng = consistent_rng();

    loop {
        let mut board = ScrabbleBoard::new(
            ScrabbleGrid::default(),
            Player::A,
            PlayerBox::new(rand_deck(MAX_DECK_SIZE, &mut rng), rand_deck(MAX_DECK_SIZE, &mut rng)),
            PlayerBox::new(0, 0),
            0,
            set.clone(),
        );
        assert!(!board.is_done());

        loop {
            // fill deck if possible
            let tiles_left = 100 - board.grid().letters_placed();

            let mut decks = board.decks();
            let deck = &mut decks[board.next_player()];

            let tiles_to_add = min(tiles_left as usize, MAX_DECK_SIZE - deck.tile_count() as usize);
            for _ in 0..tiles_to_add {
                deck.add(Letter::from_char(rng.gen_range('A'..='Z')).unwrap());
            }

            board.set_decks(decks);

            // play available move
            let moves: Vec<Move> = match board.available_moves() {
                Ok(moves) => moves.collect(),
                Err(_) => break,
            };
            let mv = *moves.choose(&mut rng).unwrap();
            board.play(mv).unwrap();
            mv_count += 1;
        }

        if mv_count >= 1_000 {
            let tp = mv_count as f64 / start.elapsed().as_secs_f64();
            println!("{} moves/sec", tp);
            mv_count = 0;
            start = Instant::now();
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
    let fst = set.as_fst();

    println!("map len: {}", set.len());
    println!("map bytes: {}", fst.as_bytes().len());

    std::fs::write("ignored/fst.bin", fst.as_bytes()).unwrap();
}

fn summarize_nodes(set: &Set) {
    let fst = set.as_fst();

    let mut nodes = HashSet::new();
    let mut todo = vec![fst.root().addr()];

    while let Some(curr) = todo.pop() {
        if nodes.insert(curr) {
            let node = fst.node(curr);
            for trans in node.transitions() {
                todo.push(trans.addr);
            }
        }
    }

    let mut final_count: u64 = 0;
    let mut total_transitions: u64 = 0;
    let mut trans_to_empty_terminal: u64 = 0;

    let mut count_per_trans = vec![0u64; 32];

    for &node in &nodes {
        let node = fst.node(node);
        let trans_count = node.transitions().count();

        final_count += node.is_final() as u64;
        total_transitions += trans_count as u64;

        count_per_trans[trans_count] += 1;

        for trans in node.transitions() {
            let dest = fst.node(trans.addr);
            if dest.is_empty() && dest.is_final() {
                trans_to_empty_terminal += 1;
            }
        }
    }

    println!("Current mem usage:");
    println!("  bytes: {}", fst.as_bytes().len());

    println!("Node summary:");
    println!("  total: {}", nodes.len());
    println!("  final: {}", final_count);
    println!("  avg transitions: {}", total_transitions as f64 / nodes.len() as f64);
    println!("  trans_to_empty_terminal: {}", trans_to_empty_terminal);

    println!("  transitions:");
    for (i, c) in count_per_trans.iter().enumerate() {
        println!("    #trans {}: {}", i, c);
    }

    let bytes_for_transitions = 4 * total_transitions;
    println!("Expected byte usage:");
    println!("  trans: {}", bytes_for_transitions);
}

fn rand_deck(len: usize, rng: &mut impl Rng) -> Deck {
    let mut deck = Deck::default();
    for _ in 0..len {
        deck.add(Letter::from_char(rng.gen_range('A'..='Z')).unwrap());
    }
    deck
}
