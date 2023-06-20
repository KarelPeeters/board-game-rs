use internal_iterator::InternalIterator;

use board_game::games::scrabble::basic::Deck;
use board_game::games::scrabble::grid::ScrabbleGrid;

type Set = fst::Set<Vec<u8>>;

const LETTERS: &str = "
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
    let set = load_fst();

    let mut grid = ScrabbleGrid::from_str_2d(&set, LETTERS.trim()).unwrap();
    grid.copy_multipliers_from(&ScrabbleGrid::standard());

    println!("{}", grid);

    grid.assert_valid(&set);

    for y in 0..grid.height {
        for x in 0..grid.width {
            println!("({}, {}) => {:?}", x, y, grid.cell(x, y));
        }
    }

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
