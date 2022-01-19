use internal_iterator::InternalIterator;
use rand::thread_rng;

use board_game::board::{Board, BoardMoves};
use board_game::games::connect4::Connect4;

fn main() {
    let mut board = Connect4::default();

    while !board.is_done() {
        println!("{}", board);
        println!("{:?}", board.available_moves().collect::<Vec<_>>());

        let mv = board.random_available_move(&mut thread_rng());
        println!("{}", mv);
        board.play(mv);
    }

    print!("{}", board);
    println!("{:?}", board.outcome());
}
