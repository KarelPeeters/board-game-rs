#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Command<'a> {
    Uai,
    IsReady,
    NewGame,
    Quit,
    Takeback,
    Print,
    Position {
        position: Position<'a>,
        moves: Option<&'a str>,
    },
    Go(GoTimeSettings),
    SetOption {
        name: &'a str,
        value: &'a str,
    },
    Moves(&'a str),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GoTimeSettings {
    Move(u32),
    Clock {
        b_time: u32,
        w_time: u32,
        b_inc: u32,
        w_inc: u32,
    },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Position<'a> {
    StartPos,
    Fen(&'a str),
}

impl<'a> Command<'a> {
    pub fn parse(input: &'a str) -> Result<Command, nom::Err<nom::error::Error<&str>>> {
        parse::command()(input).map(|(left, command)| {
            assert!(left.is_empty());
            command
        })
    }
}

mod parse {
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_until, take_while};
    use nom::character::complete::digit1;
    use nom::combinator::{eof, map, opt, value};
    use nom::sequence::{preceded, terminated, tuple};
    use nom::IResult;

    use super::*;

    pub fn command<'a>() -> impl FnMut(&'a str) -> IResult<&'a str, Command<'a>> {
        let int = || map(digit1, |s: &str| s.parse().unwrap());

        let move_time = preceded(tag("movetime "), map(int(), GoTimeSettings::Move));

        let clock_time = map(
            tuple((
                tag("btime "),
                int(),
                tag(" wtime "),
                int(),
                tag(" binc "),
                int(),
                tag(" winc "),
                int(),
            )),
            |(_, b_time, _, w_time, _, b_inc, _, w_inc)| GoTimeSettings::Clock {
                b_time,
                w_time,
                b_inc,
                w_inc,
            },
        );

        let go = preceded(tag("go "), map(alt((move_time, clock_time)), Command::Go));

        let position = map(
            tuple((
                tag("position "),
                alt((
                    value(Position::StartPos, tag("startpos")),
                    preceded(
                        tag("fen "),
                        map(alt((take_until(" moves"), take_while(|_| true))), Position::Fen),
                    ),
                )),
                opt(preceded(tag(" moves "), take_while(|_| true))),
            )),
            |(_, position, moves)| Command::Position { position, moves },
        );

        let moves = map(preceded(tag("moves "), take_while(|_| true)), |moves| {
            Command::Moves(moves)
        });

        let set_option = preceded(
            tag("setoption "),
            map(
                tuple((tag("name "), take_until(" "), tag(" value "), take_while(|_| true))),
                |(_, name, _, value)| Command::SetOption { name, value },
            ),
        );

        let main = alt((
            value(Command::NewGame, tag("uainewgame")),
            value(Command::Uai, tag("uai")),
            value(Command::IsReady, tag("isready")),
            value(Command::Quit, tag("quit")),
            value(Command::Takeback, tag("takeback")),
            value(Command::Print, alt((tag("print"), tag("d")))),
            position,
            moves,
            go,
            set_option,
        ));

        terminated(main, eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert_eq!(Ok(Command::Uai), Command::parse("uai"));
        assert_eq!(Ok(Command::IsReady), Command::parse("isready"));
        assert_eq!(Ok(Command::NewGame), Command::parse("uainewgame"));
        assert_eq!(Ok(Command::Quit), Command::parse("quit"));
    }

    #[test]
    fn moves() {
        assert_eq!(Ok(Command::Moves("a b c")), Command::parse("moves a b c"));
    }

    #[test]
    fn position() {
        assert_eq!(
            Ok(Command::Position {
                position: Position::StartPos,
                moves: None,
            }),
            Command::parse("position startpos")
        );

        assert_eq!(
            Ok(Command::Position {
                position: Position::Fen("x5o/2o2o1/7/7/4x2/5xx/o6 x 1 4"),
                moves: None,
            }),
            Command::parse("position fen x5o/2o2o1/7/7/4x2/5xx/o6 x 1 4")
        )
    }

    #[test]
    fn position_moves() {
        assert_eq!(
            Ok(Command::Position {
                position: Position::StartPos,
                moves: Some("a b c"),
            }),
            Command::parse("position startpos moves a b c")
        );

        assert_eq!(
            Ok(Command::Position {
                position: Position::Fen("x5o/2o2o1/7/7/4x2/5xx/o6 x 1 4"),
                moves: Some("a b c"),
            }),
            Command::parse("position fen x5o/2o2o1/7/7/4x2/5xx/o6 x 1 4 moves a b c")
        )
    }
}
