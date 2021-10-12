use board_game::ai::solver::is_double_forced_draw;
use board_game::games::dummy::DummyGame;

#[test]
fn draw0() {
    let board: DummyGame = "=".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), Some(true));
    assert_eq!(is_double_forced_draw(&board, 1), Some(true));
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn won0() {
    let board: DummyGame = "A".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), Some(false));
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw1() {
    let board: DummyGame = "(=)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(true));
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn won1() {
    let board: DummyGame = "(A)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw2() {
    let board: DummyGame = "((=))".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn won2() {
    let board: DummyGame = "((A))".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw1_draw1() {
    let board: DummyGame = "(==)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(true));
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn won1_draw1() {
    let board: DummyGame = "(A=)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw1_won1() {
    let board: DummyGame = "(=A)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw1_draw2() {
    let board: DummyGame = "(=(=))".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn draw2_draw1() {
    let board: DummyGame = "((=)=)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(true));
    assert_eq!(is_double_forced_draw(&board, 3), Some(true));
}

#[test]
fn draw1_won2() {
    let board: DummyGame = "(=(A))".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn won2_draw1() {
    let board: DummyGame = "((A)=)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), None);
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn won1_draw2() {
    let board: DummyGame = "(A(=))".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}

#[test]
fn draw2_won1() {
    let board: DummyGame = "((=)A)".parse().unwrap();
    assert_eq!(is_double_forced_draw(&board, 0), None);
    assert_eq!(is_double_forced_draw(&board, 1), Some(false));
    assert_eq!(is_double_forced_draw(&board, 2), Some(false));
    assert_eq!(is_double_forced_draw(&board, 3), Some(false));
}
