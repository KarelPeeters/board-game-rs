#![allow(dead_code)]

use std::collections::HashSet;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

use board_game::board::{Board, Outcome, Player};
use board_game::games::ataxx::AtaxxBoard;
use board_game::pov::Pov;
use board_game::util::bitboard::BitBoard8;
use board_game::util::bits::{SubSetCountIterator, SubSetIterator};
use board_game::wdl::{OutcomeWDL, WDL};

fn main() {
    real()
}

fn manual() {
    let size = 5;
    let mask_full = BitBoard8::FULL_FOR_SIZE[size as usize];
    let stone_count = 3;

    let iter = SubSetCountIterator::new(mask_full.0, stone_count);
    for both in iter {
        println!("{:?}", BitBoard8(both));
    }
}

enum Value {
    Unknown,
    WDL(OutcomeWDL),
}

struct Buffer {
    values: Vec<u8>,
    board_size: u8,
}

impl Buffer {
    fn lookup(&self, board: &AtaxxBoard) -> Value {
        todo!()
    }

    fn set(&self, board: &AtaxxBoard, value: Value) {
        todo!()
    }
}

fn real() {
    let size = 5;
    let area = size as u32 * size as u32;
    let mask_full = BitBoard8::FULL_FOR_SIZE[size as usize];

    let expected_total_count = (0..=area)
        .map(|stones| ncr(area as u64, stones as u64) * 2u64.pow(stones))
        .sum::<u64>();
    assert_eq!(expected_total_count, 3u64.pow(area as u32));

    println!("Expected total count: {}", expected_total_count);

    let mut total_count: u64 = 0;
    let start = Instant::now();

    for stone_count in (0..=area).rev() {
        println!("Checking stone_count={}", stone_count);

        let mut board_count: u64 = 0;
        let mut done_count: u64 = 0;

        for tiles_both in SubSetCountIterator::new(mask_full.0, stone_count) {
            let tiles_both = BitBoard8(tiles_both);
            debug_assert_eq!(tiles_both.count() as u32, stone_count);

            let mut sub_count = 0;

            for tiles_a in SubSetIterator::new(tiles_both.0) {
                let tiles_a = BitBoard8(tiles_a);
                let tiles_b = tiles_both & !tiles_a;
                debug_assert_eq!((tiles_a & !tiles_both), BitBoard8::EMPTY);

                let board = AtaxxBoard::from_parts_unchecked(size, tiles_a, tiles_b, BitBoard8::EMPTY, 0, Player::A);

                board_count += 1;
                total_count += 1;
                sub_count += 1;
                if board.is_done() {
                    done_count += 1;
                }

                // assert!(all_boards.insert(board));
            }

            debug_assert_eq!(sub_count, 2u32.pow(tiles_both.count() as u32))
        }

        let progress = total_count as f64 / expected_total_count as f64;
        println!(
            "{} boards, {} done, progress: {:.4}, eta: {:?}",
            board_count,
            done_count,
            progress,
            start.elapsed() * (1.0 / progress - 1.0) as u32,
        );
    }

    assert_eq!(total_count, expected_total_count);
}

fn ncr(n: u64, r: u64) -> u64 {
    let r = r.min(n - r);
    let numer: u64 = (n - r + 1..=n).product();
    let denom: u64 = (1..=r).product();
    numer / denom
}

impl Value {
    fn to_int(&self) -> u8 {
        match self {
            Value::Unknown => 0,
            Value::WDL(wdl) => match wdl {
                OutcomeWDL::Win => 1,
                OutcomeWDL::Draw => 2,
                OutcomeWDL::Loss => 3,
            },
        }
    }

    fn from_int(i: u8) -> Self {
        match i {
            0 => Value::Unknown,
            1 => Value::WDL(OutcomeWDL::Win),
            2 => Value::WDL(OutcomeWDL::Draw),
            3 => Value::WDL(OutcomeWDL::Loss),
            _ => {
                debug_assert!(false);
                Value::Unknown
            }
        }
    }

    fn flip(&self) -> Self {
        match self {
            Value::Unknown => Value::Unknown,
            Value::WDL(wdl) => Value::WDL(wdl.flip()),
        }
    }
}
