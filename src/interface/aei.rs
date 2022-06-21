//! The _Arimaa Engine Interface_ (AEI).
//!
//! A derivative of the UCI protocol for the game Arimaa.
//! Specification available at https://github.com/Janzert/AEI/blob/master/aei-protocol.txt

use std::fmt::{Display, Formatter};

use crate::board::Player;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command {
    AEI,
    IsReady,
    NewGame,
    SetPosition { side: Player, board: String },
    SetOption { name: Option, value: String },
    MakeMove(String),
    Go { ponder: bool },
    Stop,
    Quit,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Response {
    ProtocolV1,
    Id { ty: IdType, value: String },
    AeiOk,
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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Option {
    TC,
    Opponent,
    OpponentRating,
    Rating,
    Rated,
    Event,
    Other(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TCOption {
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
            Response::Id { ty, value } => {
                assert!(!value.contains('\n'));
                write!(f, "id {} {}", ty, value)
            }
            Response::AeiOk => write!(f, "aeiok"),
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
        }
    }
}
