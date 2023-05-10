use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct Command {
    pub id: Option<u64>,
    pub name: String,
    pub args: Vec<String>,
}

macro_rules! command_kinds {
    ($($id:ident ($name:literal),)*) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub enum CommandKind {
            $($id),*
        }

        impl CommandKind {
            pub const ALL: &'static [CommandKind] = &[$(CommandKind::$id),*];

            pub fn name(self) -> &'static str {
                match self {
                    $(CommandKind::$id => $name,)*
                }
            }
        }

        impl Display for CommandKind {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.name())
            }
        }
    }
}

command_kinds!(
    // admin
    Name("name"),
    ProtocolVersion("protocol_version"),
    Version("version"),
    KnownCommand("known_command"),
    ListCommands("list_commands"),
    Quit("quit"),
    // setup
    BoardSize("boardsize"),
    ClearBoard("clear_board"),
    Komi("komi"),
    // TODO handicap?
    // FixedHandicap("fixed_handicap"),
    // PlaceFreeHandicap("place_free_handicap"),
    // SetFreeHandicap("set_free_handicap"),
    // core play
    Play("play"),
    GenMove("genmove"),
    Undo("undo"),
    // tournament
    TimeSettings("time_settings"),
    TimeLeft("time_left"),
    FinalScore("final_score"),
    FinalStatusList("final_status_list"),
    // TODO regression?
    // LoadSgf("loadsgf"),
    // RegGenMove("reg_genmove"),
    //debug
    ShowBoard("showboard"),
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FinalStatusKind {
    Alive,
    Dead,
    Seki,
    // WhiteTerritory,
    // BlackTerritory,
    // Dame,
}

pub type ResponseInner = Result<Option<String>, String>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Response {
    id: Option<u64>,
    inner: ResponseInner,
}

impl Response {
    pub fn new(id: Option<u64>, inner: ResponseInner) -> Self {
        Self { id, inner }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidCommand;

impl FromStr for Command {
    type Err = InvalidCommand;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens: VecDeque<_> = s.split(' ').collect();
        if tokens.is_empty() {
            return Err(InvalidCommand);
        }

        let id = if let Ok(id) = u64::from_str(tokens[0]) {
            tokens.pop_front();
            Some(id)
        } else {
            None
        };

        let name = tokens.pop_front().ok_or(InvalidCommand)?.to_owned();
        let args = tokens.into_iter().map(|s| s.to_owned()).collect_vec();

        Ok(Command { id, name, args })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownCommand;

impl FromStr for CommandKind {
    type Err = UnknownCommand;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.iter().find(|c| c.name() == s).ok_or(UnknownCommand).copied()
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (&self.inner, self.id) {
            (Ok(Some(c)), Some(id)) => write!(f, "={id} {c}\n\n"),
            (Ok(None), Some(id)) => write!(f, "={id}\n\n"),
            (Ok(Some(c)), None) => write!(f, "= {c}\n\n"),
            (Ok(None), None) => write!(f, "=\n\n"),
            (Err(c), Some(id)) => write!(f, "?{id} {c}\n\n"),
            (Err(c), None) => write!(f, "? {c}\n\n"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownStatus;

impl FromStr for FinalStatusKind {
    type Err = UnknownStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "alive" => Ok(FinalStatusKind::Alive),
            "dead" => Ok(FinalStatusKind::Dead),
            "seki" => Ok(FinalStatusKind::Seki),
            _ => Err(UnknownStatus),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::interface::gtp::CommandKind;
    use std::str::FromStr;

    #[test]
    fn parse_all() {
        for &kind in CommandKind::ALL {
            assert_eq!(Ok(kind), CommandKind::from_str(&kind.to_string()))
        }
    }
}
