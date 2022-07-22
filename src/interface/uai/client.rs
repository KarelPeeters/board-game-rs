use std::collections::VecDeque;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter, Read};
use std::time::Instant;

use crate::board::{Board, Player};
use crate::games::ataxx::{AtaxxBoard, Move};
use crate::interface::uai::command::{Command, GoTimeSettings, Position};

pub const MAX_STACK_SIZE: usize = 100;

pub fn run(
    mut bot: impl FnMut(&AtaxxBoard, u32) -> (Move, String),
    name: &str,
    author: &str,
    input: impl Read,
    output: impl Write,
    log: impl Write,
) -> std::io::Result<()> {
    // wrap everything
    let input = &mut BufReader::new(input);
    let mut output = &mut BufWriter::new(output);
    let log = &mut BufWriter::new(log);

    //warmup
    bot(&AtaxxBoard::default(), 1000);

    let mut line = String::new();
    let mut board_stack = VecDeque::new();

    loop {
        log.flush()?;
        output.flush()?;

        while board_stack.len() > MAX_STACK_SIZE {
            board_stack.pop_back();
        }

        line.clear();
        input.read_line(&mut line)?;
        let line = line.trim();
        writeln!(log, "> {}", line).unwrap();
        println!("> {}", line);

        let command = match Command::parse(line) {
            Ok(command) => command,
            Err(_) => {
                writeln!(output, "error: failed to parse command '{}'", line)?;
                continue;
            }
        };

        match command {
            Command::Uai => {
                writeln!(output, "id name {}", name)?;
                writeln!(output, "id author {}", author)?;
                writeln!(output, "uaiok")?;
            }
            Command::IsReady => {
                writeln!(output, "readyok")?;
            }
            Command::SetOption { name, value } => {
                writeln!(
                    output,
                    "warning: ignoring command setoption, name={}, value={}",
                    name, value
                )?;
            }
            Command::NewGame => {
                board_stack.push_front(AtaxxBoard::default());
            }
            Command::Print => match board_stack.front() {
                Some(board) => {
                    let board = board.to_string();
                    writeln!(output, "info: current board:")?;
                    for line in board.lines() {
                        writeln!(output, "info: {}", line)?;
                    }
                }
                None => writeln!(output, "info: no board")?,
            },
            Command::Takeback => {
                let popped = board_stack.pop_front().is_some();
                if !popped {
                    writeln!(output, "error: cannot takeback, board stack is empty")?;
                }
            }
            Command::Position { position, moves } => {
                let board = match position {
                    Position::StartPos => AtaxxBoard::default(),
                    Position::Fen(fen) => AtaxxBoard::from_fen(fen).unwrap(),
                };
                board_stack.push_front(board);
                if let Some(moves) = moves {
                    apply_moves(&mut output, &mut board_stack, moves)?;
                }
            }
            Command::Moves(moves) => {
                apply_moves(&mut output, &mut board_stack, moves)?;
            }
            Command::Go(time_settings) => {
                let curr_board = match board_stack.front() {
                    Some(curr_board) => curr_board,
                    None => {
                        writeln!(output, "error: received go command without having a board",)?;
                        continue;
                    }
                };

                let time_to_use = match time_settings {
                    GoTimeSettings::Move(time) => time * 95 / 100,
                    GoTimeSettings::Clock { w_time, b_time, .. } => {
                        let time_left = match curr_board.next_player() {
                            Player::A => w_time,
                            Player::B => b_time,
                        };
                        time_left / 30
                    }
                };

                writeln!(log, "time_to_use: {}", time_to_use)?;

                let start = Instant::now();
                let (best_move, info) = bot(curr_board, time_to_use);
                let time_used = (Instant::now() - start).as_secs_f32();

                writeln!(log, "best_move: {:?}, time_used: {}, {}", best_move, time_used, info)?;
                writeln!(output, "bestmove {}", best_move.to_uai())?;
            }
            Command::Quit => {
                // no nothing
            }
        }
    }
}

fn apply_moves(mut output: impl Write, board_stack: &mut VecDeque<AtaxxBoard>, moves: &str) -> std::io::Result<()> {
    let mut curr_board = match board_stack.front() {
        None => {
            writeln!(output, "error: received moves command without having a board")?;
            return Ok(());
        }
        Some(board) => board.clone(),
    };

    for mv in moves.trim().split(' ') {
        let mv = mv.trim();
        if mv.len() != 0 {
            let mv = match Move::from_uai(mv) {
                Ok(mv) => mv,
                Err(_) => {
                    writeln!(output, "error: invalid move '{}'", mv)?;
                    return Ok(());
                }
            };

            if curr_board.is_done() {
                writeln!(output, "error: cannot play move '{}', board is already done", mv)?;
                return Ok(());
            }
            if !curr_board.is_available_move(mv) {
                writeln!(output, "error: move '{}' is not available", mv)?;
                return Ok(());
            }

            curr_board.play(mv);
            board_stack.push_front(curr_board.clone());
        }
    }

    Ok(())
}
