use std::fmt::Debug;
use std::time::Instant;

use board_game::board::Board;
use board_game::util::game_stats;

pub mod ataxx;
pub mod chess;

pub fn perft_main<S: Debug + ?Sized, T: Debug, B: Board>(
    f: impl Fn(&S) -> B,
    r: Option<impl Fn(&B) -> T>,
    cases: Vec<(&S, Vec<u64>)>,
) where
    for<'a> &'a S: PartialEq<T>,
{
    let total_start = Instant::now();

    for (desc, expected_perfts) in cases {
        let board = f(desc);
        println!("Parsed {:?} as", desc);
        println!("{}", board);

        if let Some(r) = &r {
            assert_eq!(desc, r(&board), "Description mismatch");
        }

        for (depth, &expected_perft) in expected_perfts.iter().enumerate() {
            let curr_start = Instant::now();
            let perft = game_stats::perft(&board, depth as u32);
            println!(
                "   depth {} -> {} =? {}, took {:?}",
                depth,
                expected_perft,
                perft,
                curr_start.elapsed()
            );
            assert_eq!(expected_perft, perft)
        }
    }

    println!("Total: took {:?}", total_start.elapsed());
}
