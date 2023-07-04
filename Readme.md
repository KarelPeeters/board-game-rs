# board-game-rs

[![Crates.io](https://img.shields.io/crates/v/board-game)](https://crates.io/crates/board-game)
[![CI status](https://github.com/KarelPeeters/board-game-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/KarelPeeters/board-game-rs/actions)

<!--
Everything within the cargo-rdme comments is autogenerated based on the crate-level docs in lib.rs.
DO NOT EDIT MANUALLY
-->

<!-- cargo-rdme start -->

A [Board](https://docs.rs/board-game/latest/board_game/board/trait.Board.html) abstraction for deterministic two player games.
This allows for code to be generic over the actual game, so it only needs to written once.

## Content

Currently, the implemented games are:
* [Chess](https://en.wikipedia.org/wiki/Chess) as [ChessBoard](https://docs.rs/board-game/latest/board_game/games/chess/struct.ChessBoard.html),
    implemented as a simple wrapper around the [chess](https://crates.io/crates/chess) crate.
* [Go/Baduk](https://en.wikipedia.org/wiki/Go_(game))
    as [GoBoard](https://docs.rs/board-game/latest/board_game/games/go/struct.GoBoard.html).
* [Super/Ultimate tic-tac-toe](https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe)
    as [STTTBoard](https://docs.rs/board-game/latest/board_game/games/sttt/struct.STTTBoard.html).
* [Ataxx](https://en.wikipedia.org/wiki/Ataxx)
    as [AtaxxBoard](https://docs.rs/board-game/latest/board_game/games/ataxx/struct.AtaxxBoard.html).
* [Oware](https://en.wikipedia.org/wiki/Oware) as [OwareBoard](https://docs.rs/board-game/latest/board_game/games/oware/struct.OwareBoard.html).
* [Connect4](https://en.wikipedia.org/wiki/Connect_Four) as [Connect4](https://docs.rs/board-game/latest/board_game/games/connect4/struct.Connect4.html).
* [Tic Tac Toe](https://en.wikipedia.org/wiki/Tic-tac-toe) as [TTTBoard](https://docs.rs/board-game/latest/board_game/games/ttt/struct.TTTBoard.html).

Most game implementations are heavily optimized, using bitboards or other techniques where appropriate.

There are also some utility boards:
* [MaxMovesBoard](https://docs.rs/board-game/latest/board_game/games/max_length/struct.MaxMovesBoard.html)
    wraps another board and sets the outcome to a draw after move limit has been reached.
* [DummyGame](https://docs.rs/board-game/latest/board_game/games/dummy/struct.DummyGame.html)
    is a board that is constructed from an explicit game tree, useful for debugging.

Utilities in this crate that work for any [Board](https://docs.rs/board-game/latest/board_game/board/trait.Board.html):
* Game-playing algorithms, specifically:
    * [RandomBot](https://docs.rs/board-game/latest/board_game/ai/simple/struct.RandomBot.html),
        which simply picks a random move.
    * [RolloutBot](https://docs.rs/board-game/latest/board_game/ai/simple/struct.RolloutBot.html),
        which simulates a fixed number of random games for each possible move and picks the one with the best win probability.
    * [MinimaxBot](https://docs.rs/board-game/latest/board_game/ai/minimax/struct.MiniMaxBot.html),
        which picks the best move as evaluated by a customizable heuristic at a fixed depth. (implemented as alpha-beta negamax).
    * [MCTSBot](https://docs.rs/board-game/latest/board_game/ai/mcts/struct.MCTSBot.html),
        which picks the best move as found by [Monte Carlo Tree Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search).
* Random board generation functions, see [board_gen](https://docs.rs/board-game/latest/board_game/util/board_gen/).
* A bot vs bot game runner to compare playing strength, see [bot_game](https://docs.rs/board-game/latest/board_game/util/bot_game/).
* Simple game statistics (perft, random game length) which can be used to test board implementations.

This crate is also used as the foundation for [kZero](https://github.com/KarelPeeters/kZero),
a general AlphaZero implementation.

## Examples

### List the available moves on a board and play a random one.

```rust
let mut board = AtaxxBoard::default();
println!("{}", board);

board.available_moves().unwrap().for_each(|mv| {
    println!("{:?}", mv)
});

let mv = board.random_available_move(&mut rng).unwrap();
println!("Picked move {:?}", mv);
board.play(mv).unwrap();
println!("{}", board);
```

### Get the best move according to MCTS

```rust
let board = AtaxxBoard::default();
println!("{}", board);

let mut bot = MCTSBot::new(1000, 2.0, thread_rng());
println!("{:?}", bot.select_move(&board))
```

<!-- cargo-rdme end -->
