use board_game::solve_ataxx;

fn main() {
    std::thread::Builder::new()
        .stack_size(1024 * 1024 * 1024)
        .spawn(solve_ataxx::main)
        .unwrap()
        .join()
        .unwrap()
}
