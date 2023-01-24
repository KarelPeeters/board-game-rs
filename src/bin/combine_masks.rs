use itertools::Itertools;

use board_game::util::bitboard::BitBoard8;
use board_game::util::mask;
use board_game::util::mask::Operation;

fn main() {
    let result_mask = BitBoard8::FULL_FOR_SIZE[8].0;

    let reqs = mask::find_requirements(OPS, result_mask);
    println!("Reqs:");
    for (req_index, &req) in reqs.iter().enumerate() {
        println!("{}", req_index);
        println!("{}", req);
    }

    let actual = mask::cover_masks(&reqs);
    println!("Actual:");
    for (mask, map) in &actual {
        println!("{:?}", map);
        println!("shifts: {:?}", map.iter().map(|&i| OPS[i].0).collect_vec());
        println!("{:#x}", mask.one());
        println!("{}", mask);
    }
}

#[allow(dead_code)]
const OPS: &[(i32, Operation)] = &[
    (-1 + 8, |b| b.left().up()),
    (1 + 8, |b| b.right().up()),
    (-1 - 8, |b| b.left().down()),
    (1 - 8, |b| b.right().down()),
    (-1, |b| b.left()),
    (1, |b| b.right()),
    (-8, |b| b.down()),
    (8, |b| b.up()),
];

#[allow(dead_code)]
const OPS_RING: &[(i32, Operation)] = &[
    (-1 - 1, |b| b.left().left()),
    (-1 - 1 - 8, |b| b.left().left().down()),
    (-1 - 1 - 8 - 8, |b| b.left().left().down().down()),
    (-1 - 8 - 8, |b| b.left().down().down()),
    (-8 - 8, |b| b.down().down()),
    (1 - 8 - 8, |b| b.right().down().down()),
    (1 + 1 - 8 - 8, |b| b.right().right().down().down()),
    (1 + 1 - 8, |b| b.right().right().down()),
    (1 + 1, |b| b.right().right()),
    (1 + 1 + 8, |b| b.right().right().up()),
    (1 + 1 + 8 + 8, |b| b.right().right().up().up()),
    (1 + 8 + 8, |b| b.right().up().up()),
    (8 + 8, |b| b.up().up()),
    (-1 + 8 + 8, |b| b.left().up().up()),
    (-1 - 1 + 8 + 8, |b| b.left().left().up().up()),
    (-1 - 1 + 8, |b| b.left().left().up()),
];
