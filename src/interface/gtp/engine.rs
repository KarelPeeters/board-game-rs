use std::cmp::Ordering;
use std::io::{BufRead, Write};
use std::str::FromStr;

use itertools::Itertools;
use nohash_hasher::IntSet;

use crate::board::{Board, BoardDone, PlayError, Player};
use crate::games::go::{go_player_from_symbol, Chains, GoBoard, Komi, Move, Rules, State, Tile, Zobrist, GO_MAX_SIZE};
use crate::interface::gtp::command::{Command, CommandKind, FinalStatusKind, Response, ResponseInner};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Action {
    Resign,
    Move(Move),
}

pub trait GtpBot {
    fn select_action(&mut self, board: &GoBoard, time: &TimeInfo, log: &mut impl Write) -> Result<Action, BoardDone>;
}

#[derive(Debug)]
pub struct GtpEngineState {
    size: u8,
    rules: Rules,
    komi: Komi,
    time_settings: TimeSettings,
    time_left_a: TimeLeft,
    time_left_b: TimeLeft,

    state: BoardState,
    stack: Vec<BoardState>,
}

#[derive(Debug, Copy, Clone)]
pub struct TimeInfo {
    pub settings: TimeSettings,
    pub time_left: TimeLeft,
    pub time_left_opponent: TimeLeft,
}

/// See <http://www.lysator.liu.se/~gunnar/gtp/gtp2-spec-draft2/gtp2-spec.html#sec:time-handling>
#[derive(Debug, Copy, Clone)]
pub struct TimeSettings {
    pub main_time: u32,
    pub byo_yomi_time: u32,
    pub byo_yomi_stones: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct TimeLeft {
    pub time_left: u32,
    pub stones_left: u32,
}

#[derive(Debug)]
struct BoardState {
    chains: Chains,
    state: State,

    #[allow(dead_code)]
    captured_a: u32,
    #[allow(dead_code)]
    captured_b: u32,

    history: IntSet<Zobrist>,

    // only used for board printing
    prev_player: Player,
}

impl GtpEngineState {
    pub fn new() -> Self {
        let size = 19;
        GtpEngineState {
            size,
            komi: Komi::zero(),
            rules: Rules::cgos(),
            state: BoardState::new(size),
            stack: vec![],
            time_settings: TimeSettings {
                main_time: 0,
                byo_yomi_time: 0,
                byo_yomi_stones: 0,
            },
            time_left_a: TimeLeft {
                time_left: 5,
                stones_left: 1,
            },
            time_left_b: TimeLeft {
                time_left: 5,
                stones_left: 1,
            },
        }
    }

    fn clear(&mut self) {
        self.state = BoardState::new(self.size);
        self.stack.clear();
    }

    fn arbitrary(&mut self) {
        self.clear();
    }

    fn board(&self, next_player: Player) -> GoBoard {
        GoBoard::from_parts(
            self.rules,
            self.state.chains.clone(),
            next_player,
            self.state.state,
            self.state.history.clone(),
            self.komi,
        )
    }

    fn time_info(&self, player: Player) -> TimeInfo {
        let (time_left, time_left_opponent) = match player {
            Player::A => (self.time_left_a, self.time_left_b),
            Player::B => (self.time_left_b, self.time_left_a),
        };

        TimeInfo {
            settings: self.time_settings,
            time_left,
            time_left_opponent,
        }
    }

    fn play(&mut self, player: Player, mv: Move) -> Result<(), PlayError> {
        let mut board = self.board(player);
        board.play(mv)?;

        let new_state = BoardState {
            chains: board.chains().clone(),
            state: board.state(),
            captured_a: captured(Player::A, player, &self.state.chains, board.chains()),
            captured_b: captured(Player::B, player, &self.state.chains, board.chains()),
            history: board.history().clone(),
            prev_player: player,
        };
        let old_state = std::mem::replace(&mut self.state, new_state);
        self.stack.push(old_state);

        Ok(())
    }

    fn handle_command(&mut self, command: &Command, engine: &mut impl GtpBot, log: &mut impl Write) -> ResponseInner {
        let kind = CommandKind::from_str(&command.name);
        // TODO find nice way to handle command arg length checking

        match kind {
            Ok(CommandKind::Name) => {
                check_arg_count(command, 0)?;
                Ok(Some("kZero".to_string()))
            }
            Ok(CommandKind::ProtocolVersion) => {
                check_arg_count(command, 0)?;
                Ok(Some("2".to_string()))
            }
            Ok(CommandKind::Version) => {
                check_arg_count(command, 0)?;
                Ok(Some("0.1.0".to_string()))
            }
            Ok(CommandKind::KnownCommand) => {
                check_arg_count(command, 1)?;
                let command_name = &command.args[0];
                let known = CommandKind::from_str(command_name).is_ok();
                Ok(Some(known.to_string()))
            }
            Ok(CommandKind::ListCommands) => {
                check_arg_count(command, 0)?;
                let list = CommandKind::ALL.iter().map(|c| format!("{}", c)).join("\n");
                Ok(Some(list))
            }
            Ok(CommandKind::Quit) => unreachable!(),
            Ok(CommandKind::BoardSize) => {
                check_arg_count(command, 1)?;
                let new_size = &command.args[0];

                if let Ok(new_size) = u8::from_str(new_size) {
                    if new_size <= GO_MAX_SIZE {
                        self.size = new_size;
                        self.arbitrary();
                        return Ok(None);
                    }
                }

                Err("unacceptable size".to_string())
            }
            Ok(CommandKind::ClearBoard) => {
                check_arg_count(command, 0)?;
                self.clear();
                Ok(None)
            }
            Ok(CommandKind::Komi) => {
                check_arg_count(command, 1)?;
                let new_komi = &command.args[0];
                match Komi::from_str(new_komi) {
                    Ok(new_komi) => {
                        self.komi = new_komi;
                        Ok(None)
                    }
                    Err(_) => Err("syntax error".to_string()),
                }
            }
            Ok(CommandKind::Play) => {
                check_arg_count(command, 2)?;
                let color = &command.args[0];
                let vertex = &command.args[1];

                let player = player_from_color(color)?;
                let mv = match Move::from_str(vertex) {
                    Err(_) => return Err("invalid move".to_string()),
                    Ok(tile) => tile,
                };

                match self.play(player, mv) {
                    Ok(()) => Ok(None),
                    Err(_) => Err("illegal move".to_string()),
                }
            }
            Ok(CommandKind::GenMove) => {
                check_arg_count(command, 1)?;
                let color = &command.args[0];
                let player = player_from_color(color)?;

                let board = self.board(player);
                let time_info = self.time_info(player);

                let action = engine
                    .select_action(&board, &time_info, log)
                    .map_err(|_| "board done".to_string())?;

                let vertex = match action {
                    Action::Move(mv) => {
                        self.play(player, mv).unwrap();
                        match mv {
                            Move::Pass => "pass".to_string(),
                            Move::Place(tile) => tile.to_string(),
                        }
                    }
                    Action::Resign => "resign".to_string(),
                };
                Ok(Some(vertex))
            }
            Ok(CommandKind::Undo) => {
                check_arg_count(command, 0)?;
                if let Some(old_state) = self.stack.pop() {
                    self.state = old_state;
                    Ok(None)
                } else {
                    Err("cannot undo".to_string())
                }
            }
            Ok(CommandKind::TimeSettings) => {
                check_arg_count(command, 3)?;

                let main_time = u32::from_str(&command.args[0]).map_err(|_| "syntax error".to_string())?;
                let byo_yomi_time = u32::from_str(&command.args[1]).map_err(|_| "syntax error".to_string())?;
                let byo_yomi_stones = u32::from_str(&command.args[2]).map_err(|_| "syntax error".to_string())?;

                self.time_settings.main_time = main_time;
                self.time_settings.byo_yomi_time = byo_yomi_time;
                self.time_settings.byo_yomi_stones = byo_yomi_stones;

                Ok(None)
            }
            Ok(CommandKind::TimeLeft) => {
                check_arg_count(command, 3)?;

                let color = &command.args[0];
                let player = player_from_color(color)?;

                let time_left = u32::from_str(&command.args[1]).map_err(|_| "syntax error".to_string())?;
                let stones_left = u32::from_str(&command.args[2]).map_err(|_| "syntax error".to_string())?;

                let time_left = TimeLeft { time_left, stones_left };
                match player {
                    Player::A => self.time_left_a = time_left,
                    Player::B => self.time_left_b = time_left,
                }

                Ok(None)
            }
            Ok(CommandKind::FinalScore) => {
                let score = self.state.chains.score();
                let str = match score.a.cmp(&score.b) {
                    Ordering::Equal => "0".to_string(),
                    Ordering::Greater => format!("B+{}", score.a - score.b),
                    Ordering::Less => format!("W+{}", score.b - score.a),
                };
                Ok(Some(str))
            }
            Ok(CommandKind::FinalStatusList) => {
                check_arg_count(command, 1)?;
                let kind = &command.args[0];
                let kind = match FinalStatusKind::from_str(kind) {
                    Ok(kind) => kind,
                    Err(_) => return Err("invalid status".to_string()),
                };

                let stones = match kind {
                    // we consider all existing stones alive
                    FinalStatusKind::Alive | FinalStatusKind::Seki => {
                        let chains = &self.state.chains;
                        Tile::all(self.size)
                            .filter(|&tile| chains.stone_at(tile.to_flat(chains.size())).is_some())
                            .collect_vec()
                    }
                    // we don't consider any stones dead
                    FinalStatusKind::Dead => vec![],
                };

                let list = stones.iter().map(|&tile| tile.to_string()).join("\n");
                Ok(Some(list))
            }
            Ok(CommandKind::ShowBoard) => {
                let board = self.board(self.state.prev_player.other());
                let board_str = board.to_string();
                Ok(Some(board_str))
            }
            Err(_) => Err("unknown command".to_string()),
        }
    }

    pub fn run_loop(
        &mut self,
        mut engine: impl GtpBot,
        input: impl BufRead,
        mut output: impl Write,
        mut log: impl Write,
    ) -> std::io::Result<()> {
        for line in input.lines() {
            let line = match line {
                Ok(line) => line,
                // the input stream disconnecting is fine
                Err(_) => break,
            };

            let line = preprocess_input(&line);
            let line = line.trim_end();

            writeln!(&mut log, "> {}", line)?;
            log.flush()?;

            if let Ok(command) = Command::from_str(line) {
                // handle quit command here
                if command.name == "quit" {
                    break;
                }

                let id = command.id;
                let inner = self.handle_command(&command, &mut engine, &mut log);
                let response = Response::new(id, inner);

                // the output stream disconnecting is not really an error
                // no newlines, already included in response
                if write!(&mut output, "{}", response).is_err() {
                    break;
                }
                if output.flush().is_err() {
                    break;
                }

                writeln!(&mut log, "< {}", response.to_string().trim_end_matches("\n\n"))?;
                log.flush()?;
            }
        }

        Ok(())
    }
}

impl BoardState {
    pub fn new(size: u8) -> Self {
        BoardState {
            chains: Chains::new(size),
            state: State::Normal,
            captured_a: 0,
            captured_b: 0,
            history: Default::default(),
            prev_player: Player::B, // assume A plays first
        }
    }
}

fn captured(target: Player, prev: Player, before: &Chains, after: &Chains) -> u32 {
    let expected = before.stone_count_from(target) as u32 + (target == prev) as u32;
    let actual = after.stone_count_from(target) as u32;
    assert!(expected >= actual);
    expected - actual
}

fn player_from_color(s: &str) -> Result<Player, String> {
    match s.to_lowercase().as_str() {
        "black" => return Ok(Player::A),
        "white" => return Ok(Player::B),
        s if s.len() == 1 => {
            if let Some(player) = go_player_from_symbol(s.chars().next().unwrap()) {
                return Ok(player);
            }
        }
        _ => {}
    }

    Err("invalid color".to_string())
}

fn check_arg_count(command: &Command, count: usize) -> Result<(), String> {
    if command.args.len() == count {
        Ok(())
    } else {
        Err(format!(
            "wrong arg count, expected {} got {}",
            count,
            command.args.len()
        ))
    }
}

fn preprocess_input(before: &str) -> String {
    assert!(before.is_ascii());

    let cleaned = before.replace(|c: char| c.is_ascii_control() && c != '\n' && c != '\t', "");

    let mut result = String::new();

    for line in cleaned.lines() {
        if line.starts_with('#') || line.chars().all(|c| c.is_ascii_whitespace()) {
            continue;
        };
        result.push_str(&line.replace('\t', " "));
        result.push('\n');
    }

    result
}

impl TimeInfo {
    /// *Warning*: returns inf if there are no time limits
    pub fn simple_time_to_use(&self, expected_stones_left: f32) -> f32 {
        // TODO consider byo_yomi
        let expected_stones_left = f32::max(2.0, expected_stones_left);
        self.time_left.time_left as f32 / expected_stones_left
    }
}
