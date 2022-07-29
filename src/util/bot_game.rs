//! Utilities to run bots against each other and report the results.
use std::fmt::Write;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use std::time::Instant;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::ai::Bot;
use crate::board::{Board, Outcome, Player};
use crate::pov::NonPov;
use crate::util::rating::elo_from_wdl;
use crate::wdl::WDL;

/// Run `bot_l` against `bot_r` against each other on the board given by `start`.
///
/// `games_per_side` games are run, except if `both_sides` is true, in
/// which case a match consists of two games per start position where players switch sides.
///
/// Progress indications can be displayed at intervals of `print_progress_every`.
#[must_use]
pub fn run<B: Board, L: Bot<B>, R: Bot<B>>(
    start: impl Fn() -> B + Sync,
    bot_l: impl Fn() -> L + Sync,
    bot_r: impl Fn() -> R + Sync,
    games_per_side: u32,
    both_sides: bool,
    callback: impl Fn(WDL<u32>, &Replay<B>) + Sync,
) -> BotGameResult<B> {
    let callback = &callback;

    // this instantiates both at least once so we catch errors before starting a bunch of threads
    let debug_l = debug_to_string(&bot_l());
    let debug_r = debug_to_string(&bot_r());

    let game_count = if both_sides { 2 * games_per_side } else { games_per_side };
    let starts = (0..games_per_side).map(|_| start()).collect_vec();

    let partial_wdl = Mutex::new(WDL::<u32>::default());

    let replays: Vec<Replay<B>> = (0..game_count)
        .into_par_iter()
        .panic_fuse()
        .map(|game_i| {
            let flip = if both_sides { game_i % 2 == 1 } else { false };
            let pair_i = if both_sides { game_i / 2 } else { game_i };
            let start = &starts[pair_i as usize];

            let replay = play_single_game(start, flip, &mut bot_l(), &mut bot_r());

            let mut partial_wdl = partial_wdl.lock().unwrap();
            *partial_wdl += replay.outcome.pov(replay.player_l).to_wdl();
            callback(*partial_wdl, &replay);

            replay
        })
        .collect();

    let total_time_l = replays.iter().map(|r| r.total_time_l).sum::<f32>();
    let total_time_r = replays.iter().map(|r| r.total_time_r).sum::<f32>();
    let move_count_l = replays.iter().map(|r| r.move_count_l).sum::<u32>();
    let move_count_r = replays.iter().map(|r| r.move_count_r).sum::<u32>();

    BotGameResult {
        game_count,
        average_game_length: replays.iter().map(|r| r.moves.len() as f32).sum::<f32>() / game_count as f32,
        wdl_l: replays.iter().map(|r| r.outcome.pov(r.player_l).to_wdl()).sum(),
        time_l: total_time_l / move_count_l as f32,
        time_r: total_time_r / move_count_r as f32,
        debug_l,
        debug_r,
        replays,
    }
}

fn play_single_game<B: Board>(start: &B, flip: bool, bot_l: &mut impl Bot<B>, bot_r: &mut impl Bot<B>) -> Replay<B> {
    let mut board = start.clone();
    let player_l = if flip {
        board.next_player().other()
    } else {
        board.next_player()
    };

    let mut total_time_l = 0.0;
    let mut total_time_r = 0.0;
    let mut move_count_l: u32 = 0;
    let mut move_count_r: u32 = 0;
    let mut moves = vec![];

    loop {
        match board.outcome() {
            None => {
                let start_time = Instant::now();
                let mv = if board.next_player() == player_l {
                    let mv = bot_l.select_move(&board);
                    total_time_l += start_time.elapsed().as_secs_f32();
                    move_count_l += 1;
                    mv
                } else {
                    let mv = bot_r.select_move(&board);
                    total_time_r += start_time.elapsed().as_secs_f32();
                    move_count_r += 1;
                    mv
                };

                moves.push(mv);
                board.play(mv);
            }
            Some(outcome) => {
                return Replay {
                    start: start.clone(),
                    player_l,
                    moves,
                    outcome,
                    total_time_l,
                    total_time_r,
                    move_count_l,
                    move_count_r,
                    debug_l: debug_to_string(bot_l),
                    debug_r: debug_to_string(bot_r),
                };
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Replay<B: Board> {
    pub start: B,
    pub player_l: Player,

    pub moves: Vec<B::Move>,
    pub outcome: Outcome,

    pub total_time_l: f32,
    pub total_time_r: f32,
    pub move_count_l: u32,
    pub move_count_r: u32,

    pub debug_l: String,
    pub debug_r: String,
}

/// Structure returned by the function [`run`].
pub struct BotGameResult<B: Board> {
    pub game_count: u32,
    pub replays: Vec<Replay<B>>,

    pub average_game_length: f32,
    pub wdl_l: WDL<u32>,

    //time per move in seconds
    pub time_l: f32,
    pub time_r: f32,

    pub debug_l: String,
    pub debug_r: String,
}

impl<B: Board> Debug for BotGameResult<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BotGameResult {{")?;
        writeln!(
            f,
            "  {} games, average length {}",
            self.game_count, self.average_game_length
        )?;
        writeln!(f, "  left      {:?}", self.wdl_l,)?;
        writeln!(
            f,
            "  left      {:.3?}",
            self.wdl_l.cast::<f32>() / self.game_count as f32
        )?;
        writeln!(f, "  left elo: {:.1}", elo_from_wdl(self.wdl_l.cast::<f32>()))?;
        writeln!(f, "  time_l:   {:.4}, time_r: {:.4}", self.time_l, self.time_r)?;
        writeln!(f, "  left:     {}", self.debug_l)?;
        writeln!(f, "  right:    {}", self.debug_r)?;
        writeln!(f, "}}")?;

        Ok(())
    }
}

fn debug_to_string(d: &impl Debug) -> String {
    let mut s = String::new();
    write!(&mut s, "{:?}", d).unwrap();
    s
}
