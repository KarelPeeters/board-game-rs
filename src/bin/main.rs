use board_game::board::{Board, BoardMoves};
use board_game::games::ataxx::AtaxxBoard;
use internal_iterator::InternalIterator;
use rand::Rng;

fn main() {
    // demo();
    fuzz();
}

#[derive(Default)]
struct Count {
    pos: u64,
    back: u64,
    mv: u64,
}

fn fuzz() {
    let mut rng = rand::thread_rng();

    let mut count = Count::default();

    fuzz_test(&AtaxxBoard::diagonal(7), &mut count);

    for _ in 0..1000 {
        let mut board = AtaxxBoard::diagonal(rng.gen_range(2..=8));
        fuzz_test(&board, &mut count);

        while let Ok(()) = board.play_random_available_move(&mut rng) {
            fuzz_test(&board, &mut count);
        }
    }

    println!("Visited {} random positions", count.pos);
    println!("  {} mv/pos", count.mv as f64 / count.pos as f64);
    println!("  {} back/pos", count.back as f64 / count.pos as f64);
}

fn fuzz_test(board: &AtaxxBoard, count: &mut Count) {
    if board.is_done() {
        return;
    }

    // println!("{}", board);
    count.pos += 1;

    count.mv += board.available_moves().unwrap().count() as u64;

    // check that all back moves are valid and distinct
    let mut all_back = vec![];

    board.back_moves().for_each(|back| {
        count.back += 1;

        // println!("  {:?}", back);
        let mut prev = board.clone();
        prev.play_back(back);
        board.assert_valid();

        assert!(!all_back.contains(&back));
        all_back.push(back);
    });

    // TODO check that back returns the right board
    // board.available_moves().unwrap().for_each(|mv| {
    //     println!("  {}", mv);
    //     let child = board.clone_and_play(mv).unwrap();
    //
    //     // check that the back move exists
    //     assert!(child.back_moves().any(|back| back.mv == mv));
    //
    //     let orig = child.play_back(back);
    // };
}

fn demo() {
    // let board = AtaxxBoard::diagonal(7);
    let board = AtaxxBoard::from_fen("x5o/7/7/7/7/oo5/oo4x x 0 1").unwrap();

    println!("{}", board);

    println!("Back moves:");
    board.back_moves().for_each(|mv| {
        println!("  {:?}", mv);
    })
}
