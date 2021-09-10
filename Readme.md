This is the repository of the `board-game` Rust crate.

<!-- cargo-sync-readme start -->

A [Board](https://docs.rs/board-game/latest/board-game/board/trait.Board.html) abstraction for deterministic two player games.
This code to be generic over the actual game, so it only needs to written once.

Currently the implemented games are:
* [Super/Ultimate tic-tac-toe](https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe) in the module [sttt](https://docs.rs/board-game/latest/board-game/games/sttt/).
* [Ataxx](https://en.wikipedia.org/wiki/Ataxx) in the module [ataxx](https://docs.rs/board-game/latest/board-game/games/ataxx/).

Notable things currently implemented in this crate that work for any [Board](https://docs.rs/board-game/latest/board-game/board/trait.Board.html):
* Game-playing algorithms, specifically:
    * [RandomBot](https://docs.rs/board-game/latest/board-game/ai/simple/struct.RandomBot.html),
        which simply picks a random move.
    * [RolloutBot](https://docs.rs/board-game/latest/board-game/ai/simple/struct.RolloutBot.html),
        which simulates a fixed number of random games for each possible move and picks the one with the best win probability.
    * [MinimaxBot](https://docs.rs/board-game/latest/board-game/ai/minimax/struct.MiniMaxBot.html),
        which picks the best move as evaluated by a customizable heuristic at a fixed depth. (implemented as alpha-beta negamax).
    * [MCTSBot](https://docs.rs/board-game/latest/board-game/ai/mcts/struct.MCTSBot.html),
        which picks the best move as found by [Monte Carlo Tree Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search).
* Random board generation functions, see [board_gen](https://docs.rs/board-game/latest/board-game/util/board_gen/).
* A bot vs bot game runner to compare playing strength, see [bot_game](https://docs.rs/board-game/latest/board-game/util/bot_game/).
* Simple game statistics (perft, random game length) which can be used to test [Board](https://docs.rs/board-game/latest/board-game/board/trait.Board.html) implementations.

<!-- cargo-sync-readme end -->
