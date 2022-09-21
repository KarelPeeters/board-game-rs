use std::collections::VecDeque;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter, Read};
use std::time::Instant;

use crate::board::{Board, Player};
use crate::games::ataxx::{AtaxxBoard, Move};
use crate::interface::uai::command::{Command, GoTimeSettings, Position};

pub const MAX_STACK_SIZE: usize = 100;

pub fn run(
    mut bot: impl FnMut(&AtaxxBoard, f32) -> (Move, String),
    name: &str,
    author: &str,
    input: impl Read,
    output: impl Write,
    log: impl Write,
) -> std::io::Result<()> {
    // wrap everything
    let mut input = BufReader::new(input);
    let mut output = Output {
        output: BufWriter::new(output),
        log: BufWriter::new(log),
    };

    //warmup
    bot(&AtaxxBoard::default(), 1.0);

    let mut line = String::new();
    let mut board_stack = VecDeque::new();

    loop {
        output.flush()?;

        while board_stack.len() > MAX_STACK_SIZE {
            board_stack.pop_back();
        }

        line.clear();
        input.read_line(&mut line)?;
        let line = line.trim();
        output.log(&format!("> {}", line))?;

        let command = match Command::parse(line) {
            Ok(command) => command,
            Err(_) => {
                output.respond(&format!("info (error): failed to parse command '{}'", line))?;
                continue;
            }
        };

        match command {
            Command::Uai => {
                output.respond(&format!("id name {}", name))?;
                output.respond(&format!("id author {}", author))?;
                output.respond(&format!("uaiok"))?;
            }
            Command::IsReady => {
                output.respond(&format!("readyok"))?;
            }
            Command::SetOption { name, value } => {
                output.respond(&format!(
                    "info (warning) ignoring command setoption, name={}, value={}",
                    name, value
                ))?;
            }
            Command::NewGame => {
                board_stack.push_front(AtaxxBoard::default());
            }
            Command::Print => match board_stack.front() {
                Some(board) => {
                    let board = board.to_string();
                    output.respond(&format!("info: current board:"))?;
                    for line in board.lines() {
                        output.respond(&format!("info: {}", line))?;
                    }
                }
                None => output.respond("info (error): cannot print, no board")?,
            },
            Command::Takeback => {
                let popped = board_stack.pop_front().is_some();
                if !popped {
                    output.respond(&format!("info (error): cannot takeback, board stack is empty"))?;
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
                        output.respond(&format!("info (error): received go command without having a board",))?;
                        continue;
                    }
                };

                if let Some(outcome) = curr_board.outcome() {
                    output.respond(&format!(
                        "info (error): cannot go on done board, outcome: {:?}",
                        outcome
                    ))?;
                    continue;
                }

                let time_to_use = match time_settings {
                    GoTimeSettings::Move(time) => 0.95 * (time as f32 / 1000.0),
                    GoTimeSettings::Clock { w_time, b_time, .. } => {
                        let time_left_ms = match curr_board.next_player() {
                            Player::A => w_time,
                            Player::B => b_time,
                        };
                        let time_left = time_left_ms as f32 / 1000.0;
                        time_left / 30.0
                    }
                };

                output.respond(&format!("info (info): planning to use {}s", time_to_use))?;
                output.flush()?;

                let start = Instant::now();
                let (best_move, info) = bot(curr_board, time_to_use);
                let time_used = (Instant::now() - start).as_secs_f32();

                output.respond(&format!("info (info): time used {}s", time_used))?;
                output.respond(&format!("info (info): {}", info))?;
                output.respond(&format!("bestmove {}", best_move.to_uai()))?;
            }
            Command::Quit => {
                // no nothing
            }
        }
    }
}

fn apply_moves<O: Write, L: Write>(
    output: &mut Output<O, L>,
    board_stack: &mut VecDeque<AtaxxBoard>,
    moves: &str,
) -> std::io::Result<()> {
    let mut curr_board = match board_stack.front() {
        None => {
            output.respond(&format!("error: received moves command without having a board"))?;
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
                    output.respond(&format!("error: invalid move '{}'", mv))?;
                    return Ok(());
                }
            };

            if curr_board.is_done() {
                output.respond(&format!("error: cannot play move '{}', board is already done", mv))?;
                return Ok(());
            }
            if !curr_board.is_available_move(mv) {
                output.respond(&format!("error: move '{}' is not available", mv))?;
                return Ok(());
            }

            curr_board.play(mv);
            board_stack.push_front(curr_board.clone());
        }
    }

    Ok(())
}

struct Output<O, L> {
    output: O,
    log: L,
}

impl<O: Write, L: Write> Output<O, L> {
    fn respond(&mut self, s: &str) -> std::io::Result<()> {
        assert!(!s.contains('\n'), "UAI response cannot contain newline");
        writeln!(&mut self.log, "< {}", s)?;
        writeln!(&mut self.output, "{}", s)?;
        Ok(())
    }

    fn log(&mut self, s: &str) -> std::io::Result<()> {
        writeln!(&mut self.log, "{}", s)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()?;
        self.log.flush()?;
        Ok(())
    }
}
