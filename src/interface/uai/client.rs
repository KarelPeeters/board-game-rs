use std::collections::VecDeque;
use std::io::{BufRead, BufReader, BufWriter, Read};
use std::io::{ErrorKind, Write};
use std::time::Instant;

use crate::board::{Board, PlayError, Player};
use crate::games::ataxx::{AtaxxBoard, Move};
use crate::interface::uai::command::{Command, GoTimeSettings, Position};

pub const MAX_STACK_SIZE: usize = 100;

pub fn run(
    bot: impl FnMut(&AtaxxBoard, f32) -> (Move, String),
    name: &str,
    author: &str,
    input: impl Read,
    output: impl Write,
    log: impl Write,
) -> std::io::Result<()> {
    let result = run_inner(bot, name, author, input, output, log);

    if let Err(err) = &result {
        if err.kind() == ErrorKind::BrokenPipe {
            return Ok(());
        }
    }

    result
}

pub fn run_inner(
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
        let line_result = input.read_line(&mut line)?;

        // check for eof
        if line_result == 0 {
            return Ok(());
        }

        let line = line.trim();
        output.log(&format!("> {}", line))?;

        let command = match Command::parse(line) {
            Ok(command) => command,
            Err(_) => {
                output.error(&format!("failed to parse command '{}'", line))?;
                continue;
            }
        };

        match command {
            Command::Uai => {
                output.respond(&format!("id name {}", name))?;
                output.respond(&format!("id author {}", author))?;
                output.respond("uaiok")?;
            }
            Command::IsReady => {
                output.respond("readyok")?;
            }
            Command::SetOption { name, value } => {
                output.warning(&format!("ignoring command setoption, name={}, value={}", name, value))?;
            }
            Command::NewGame => {
                board_stack.push_front(AtaxxBoard::default());
            }
            Command::Print => match board_stack.front() {
                Some(board) => {
                    let board = board.to_string();
                    output.info("current board:")?;
                    for line in board.lines() {
                        output.info(line)?;
                    }
                }
                None => output.error("cannot print, no board")?,
            },
            Command::Takeback => {
                let popped = board_stack.pop_front().is_some();
                if !popped {
                    output.error("cannot takeback, board stack is empty")?;
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
                        output.error("received go command without having a board")?;
                        continue;
                    }
                };

                if let Some(outcome) = curr_board.outcome() {
                    output.error(&format!("cannot go on done board, outcome: {:?}", outcome))?;
                    continue;
                }

                let time_to_use = match time_settings {
                    GoTimeSettings::Move(time) => 0.95 * (time as f32 / 1000.0),
                    GoTimeSettings::Clock {
                        w_time,
                        b_time,
                        w_inc,
                        b_inc,
                    } => {
                        // careful: player A is black for ataxx
                        let (time_left_ms, inc_ms) = match curr_board.next_player() {
                            Player::A => (b_time, b_inc),
                            Player::B => (w_time, w_inc),
                        };

                        let time_left = time_left_ms as f32 / 1000.0;
                        let inc = inc_ms as f32 / 1000.0;

                        time_left / 30.0 + 0.95 * inc
                    }
                };

                output.info(&format!("planning to use {}s", time_to_use))?;
                output.flush()?;

                let start = Instant::now();
                let (best_move, info) = bot(curr_board, time_to_use);
                let time_used = (Instant::now() - start).as_secs_f32();

                output.info(&format!("time used {}s", time_used))?;
                if !info.is_empty() {
                    output.info(&info)?;
                }
                output.respond(&format!("bestmove {}", best_move.to_uai()))?;
            }
            Command::Quit => return Ok(()),
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
            output.respond("error: received moves command without having a board")?;
            return Ok(());
        }
        Some(board) => board.clone(),
    };

    for mv in moves.trim().split(' ') {
        let mv = mv.trim();
        if !mv.is_empty() {
            let mv = match Move::from_uai(mv) {
                Ok(mv) => mv,
                Err(_) => {
                    output.respond(&format!("error: invalid move '{}'", mv))?;
                    return Ok(());
                }
            };

            match curr_board.play(mv) {
                Err(PlayError::BoardDone) => {
                    output.respond(&format!("error: cannot play move '{}', board is already done", mv))?;
                    return Ok(());
                }
                Err(PlayError::UnavailableMove) => {
                    output.respond(&format!("error: move '{}' is not available", mv))?;
                    return Ok(());
                }
                Ok(()) => {
                    // TODO should we push after every move or just the last one?
                    board_stack.push_front(curr_board.clone());
                }
            }
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

    fn info(&mut self, msg: &str) -> std::io::Result<()> {
        self.respond(&format!("info string (info): {}", msg))?;
        Ok(())
    }

    fn warning(&mut self, msg: &str) -> std::io::Result<()> {
        self.respond(&format!("info string (warning): {}", msg))?;
        Ok(())
    }

    fn error(&mut self, msg: &str) -> std::io::Result<()> {
        self.respond(&format!("info string (error): {}", msg))?;
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
