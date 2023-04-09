use internal_iterator::InternalIterator;
use std::ops::ControlFlow;

use board_game::board::{BoardMoves, Player};
use board_game::games::ataxx::AtaxxBoard;
use board_game::games::ataxx::Move;
use board_game::util::bitboard::BitBoard8;
use board_game::util::bits::SubSetIterator;

fn main() {
    let size = 6;

    let mut max_count = None;
    let mut max_board = None;

    let expected_count = 2u64.pow((size * size) as u32);
    let mut count: u64 = 0;

    for tiles_a in SubSetIterator::new(BitBoard8::FULL_FOR_SIZE[size as usize].0) {
        if count % 100_000_000 == 0 {
            println!(
                "Progress: {} / {} = {}",
                count,
                expected_count,
                count as f32 / expected_count as f32
            );
        }
        count += 1;

        let tiles_a = BitBoard8(tiles_a);
        let empty = BitBoard8::EMPTY;
        let board = AtaxxBoard::from_parts(size, tiles_a, empty, empty, 0, Player::A);

        let count = board.available_moves().unwrap().count();
        // let count = board
        //     .available_moves()
        //     .unwrap()
        //     .filter(|mv| matches!(mv, Move::Jump { .. }))
        //     .count();

        if max_count.map_or(true, |max_count| count > max_count) {
            max_count = Some(count);
            max_board = Some(board);
        }

        // println!("{:?}", count);
    }

    println!("Max: {}", max_count.unwrap());
    println!("{}", max_board.as_ref().unwrap());

    let mut pass = 0;
    let mut single = 0;
    let mut double = 0;

    max_board.unwrap().available_moves().unwrap().try_for_each(|mv| {
        match mv {
            Move::Pass => pass += 1,
            Move::Copy { .. } => single += 1,
            Move::Jump { .. } => double += 1,
        }
        ControlFlow::<()>::Continue(())
    });

    println!("moves: {}", pass + single + double);
    println!("pass: {}", pass);
    println!("single: {}", single);
    println!("double: {}", double);
}
