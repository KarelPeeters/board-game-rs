//! The _Arimaa Engine Interface_ (AEI).
//!
//! A derivative of the UCI protocol for the game Arimaa.
//! Specification available at <https://github.com/Janzert/AEI/blob/master/aei-protocol.txt>.

use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    AEI,
    IsReady,
    NewGame,
    SetPosition(String),
    SetOption { name: OptionName, value: Option<String> },
    MakeMove(String),
    Go { ponder: bool },
    Stop,
    Quit,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Response {
    ProtocolV1,
    AeiOk,
    ReadyOk,
    Id { ty: IdType, value: String },
    BestMove(String),
    Info { ty: InfoType, value: String },
    Log(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IdType {
    Name,
    Author,
    Version,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InfoType {
    Score,
    Depth,
    Nodes,
    Pv,
    Time,
    CurrMoveNumber,
    String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OptionName {
    TC(TCOptionName),
    Opponent,
    OpponentRating,
    Rating,
    Rated,
    Event,
    Other(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TCOptionName {
    TcMove,
    TcReserve,
    TcPercent,
    TcMax,
    TcTotal,
    TcTurns,
    TcTurnTime,
    GReserve,
    SReserve,
    GUsed,
    SUsed,
    LastMoveUsed,
    MoveUsed,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::ProtocolV1 => write!(f, "protocol-version 1"),
            Response::AeiOk => write!(f, "aeiok"),
            Response::ReadyOk => write!(f, "readyok"),
            Response::Id { ty, value } => {
                assert!(!value.contains('\n'));
                write!(f, "id {} {}", ty, value)
            }
            Response::BestMove(mv) => {
                assert!(!mv.contains('\n'));
                write!(f, "bestmove {}", mv)
            }
            Response::Info { ty, value } => {
                assert!(!value.contains('\n'));
                write!(f, "info {} {}", ty, value)
            }
            Response::Log(log) => {
                assert!(!log.contains('\n'));
                write!(f, "log {}", log)
            }
        }
    }
}

impl Display for IdType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IdType::Name => write!(f, "name"),
            IdType::Author => write!(f, "author"),
            IdType::Version => write!(f, "version"),
        }
    }
}

impl Display for InfoType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InfoType::Score => write!(f, "score"),
            InfoType::Depth => write!(f, "depth"),
            InfoType::Nodes => write!(f, "nodes"),
            InfoType::Pv => write!(f, "pv"),
            InfoType::Time => write!(f, "time"),
            InfoType::CurrMoveNumber => write!(f, "currmovenumber"),
            InfoType::String => write!(f, "string"),
        }
    }
}

impl Command {
    pub fn parse(input: &str) -> Result<Command, nom::Err<nom::error::Error<&str>>> {
        parse::command()(input).map(|(left, command)| {
            assert!(left.is_empty());
            command
        })
    }
}

mod parse {
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_until, take_while};
    use nom::combinator::{eof, map, opt, value};
    use nom::sequence::{preceded, terminated, tuple};
    use nom::IResult;

    use crate::interface::aei::{Command, OptionName, TCOptionName};

    pub fn command<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, Command> {
        let remainder = || take_while(|_| true);

        let set_position = preceded(
            tag("setposition "),
            map(remainder(), |s: &str| Command::SetPosition(s.to_owned())),
        );

        const OPTION_MAP: &[(&str, OptionName)] = &[
            ("tcmove", OptionName::TC(TCOptionName::TcMove)),
            ("tcreserve", OptionName::TC(TCOptionName::TcReserve)),
            ("tcpercent", OptionName::TC(TCOptionName::TcPercent)),
            ("tcmax", OptionName::TC(TCOptionName::TcMax)),
            ("tctotal", OptionName::TC(TCOptionName::TcTotal)),
            ("tcturns", OptionName::TC(TCOptionName::TcTurns)),
            ("tcturntime", OptionName::TC(TCOptionName::TcTurnTime)),
            ("greserve", OptionName::TC(TCOptionName::GReserve)),
            ("sreserve", OptionName::TC(TCOptionName::SReserve)),
            ("gused", OptionName::TC(TCOptionName::GUsed)),
            ("sused", OptionName::TC(TCOptionName::SUsed)),
            ("lastmoveused", OptionName::TC(TCOptionName::LastMoveUsed)),
            ("moveused", OptionName::TC(TCOptionName::MoveUsed)),
            ("opponent", OptionName::Opponent),
            ("opponent_rating", OptionName::OpponentRating),
            ("rating", OptionName::Rating),
            ("rated", OptionName::Rated),
            ("event", OptionName::Event),
        ];

        let option_name = map(take_until(" "), |s: &str| {
            OPTION_MAP
                .iter()
                .find(|&&(k, _)| k == s)
                .map_or_else(|| OptionName::Other(s.to_owned()), |(_, v)| v.clone())
        });

        let set_option = map(
            tuple((
                tag("setoption name "),
                option_name,
                opt(preceded(tag(" value "), remainder())),
            )),
            |(_, name, value)| Command::SetOption {
                name,
                value: value.map(ToOwned::to_owned),
            },
        );

        let make_move = preceded(tag("makemove "), map(remainder(), |s| Command::MakeMove(s.to_owned())));

        let main = alt((
            value(Command::AEI, tag("aei")),
            value(Command::IsReady, tag("isready")),
            value(Command::NewGame, tag("newgame")),
            set_position,
            set_option,
            make_move,
            value(Command::Go { ponder: true }, tag("go ponder")),
            value(Command::Go { ponder: false }, tag("go")),
            value(Command::Stop, tag("stop")),
            value(Command::Quit, tag("quit")),
        ));

        terminated(main, eof)
    }
}

#[cfg(test)]
mod tests {
    use crate::interface::aei::{Command, OptionName};

    #[test]
    fn set_option() {
        let parsed = Command::parse("setoption name opponent_rating value 1325");
        let expected = Command::SetOption {
            name: OptionName::OpponentRating,
            value: Some("1325".to_owned()),
        };
        assert_eq!(parsed, Ok(expected))
    }
}
