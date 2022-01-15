use board_game::solve_ataxx;

const STACK_SIZE: usize = 1024 * 1024 * 1024 * 1024;

#[cfg(windows)]
fn main() {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(solve_ataxx::main)
        .unwrap()
        .join()
        .unwrap()
}

#[cfg(unix)]
fn main() {
    rlimit::setrlimit(rlimit::Resource::STACK, rlimit::RLIM_INFINITY, rlimit::RLIM_INFINITY);
    solve_ataxx::main();
}
