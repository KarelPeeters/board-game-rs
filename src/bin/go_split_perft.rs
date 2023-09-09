use std::str::FromStr;

use internal_iterator::InternalIterator;
use itertools::Itertools;

use board_game::board::Board;
use board_game::games::go::{GoBoard, Komi, Move, Rules};
use board_game::util::game_stats::perft;

fn main() {
    let args = std::env::args().skip(1).collect_vec();

    let (size_or_fen, depth, rules, moves): (&str, &str, &str, &str) = match args.as_slice() {
        [size, depth, rules] => (size, depth, rules, ""),
        [size, depth, rules, moves] => (size, depth, rules, moves),
        _ => usage(),
    };

    let size_or_fen = size_or_fen
        .parse::<u8>()
        .map_or(SizeOrFen::Fen(size_or_fen.to_string()), SizeOrFen::Size);
    let full_depth = depth
        .parse::<u32>()
        .unwrap_or_else(|_| error(&format!("Invalid depth {:?}", depth)));

    let rules = match rules {
        "cgos" => Rules::cgos(),
        "tt" => Rules::tromp_taylor(),
        _ => error(&format!("Invalid rules {:?}", rules)),
    };

    let moves = moves
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(Move::from_str(s).unwrap_or_else(|_| error(&format!("Invalid move {:?}", s))))
            }
        })
        .collect_vec();

    let mut board = match size_or_fen {
        SizeOrFen::Size(size) => GoBoard::new(size, Komi::zero(), rules),
        SizeOrFen::Fen(fen) => GoBoard::from_fen(&fen, rules)
            .unwrap_or_else(|e| error(&format!("Invalid size or fen {:?}, fen error: {:?}", fen, e))),
    };

    for &mv in &moves {
        board
            .play(mv)
            .unwrap_or_else(|e| error(&format!("Cannot play {:?}, {:?}", mv, e)));
    }

    println!("Settings");
    println!("  size {}, rules {:?}, moves {:?}", board.size(), rules, moves);
    println!("  depth: {}", depth);

    println!();
    println!("FEN: {:?}", board.to_fen());
    println!("{}", board);

    let mut total = 0;

    if moves.len() > full_depth as usize {
        println!("Warning: more moves than depth, so no children are visited");
    } else {
        let depth = full_depth - (moves.len() as u32);
        println!("Remaining depth: {}", depth);

        if depth == 0 {
            println!("Warning: remaining depth is 0, so no children are visited");
        } else if let Ok(children) = board.children() {
            println!("Children:");
            children.for_each(|(mv, child)| {
                let count = perft(&child, depth - 1);

                println!("{}: {}", mv, count);
                total += count;
            });
        } else {
            println!("Warning: board is done");
        }
    }

    println!();
    println!("Total: {}", total);

    println!();
}

fn error(str: &str) -> ! {
    eprintln!("{}", str);
    usage()
}

fn usage() -> ! {
    eprintln!("Usage: split_perft <size or fen> <depth> <rules> [moves]");
    eprintln!("  rules can be 'cgos' or 'tt'");
    eprintln!("  moves is a comma separated list of moves, eg. A4,B3,PASS");
    eprintln!("  the moves are subtracted from the effective depth");
    std::process::exit(1);
}

enum SizeOrFen {
    Size(u8),
    Fen(String),
}
