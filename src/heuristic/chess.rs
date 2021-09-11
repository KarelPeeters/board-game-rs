use crate::ai::minimax::Heuristic;
use crate::games::chess::ChessBoard;
use crate::ai::solver::SolverHeuristic;
use crate::board::Board;
use chess::{ALL_PIECES, Piece};

#[derive(Debug)]
struct ChessPieceValueHeuristic;

impl Heuristic<ChessBoard> for ChessPieceValueHeuristic {
    type V = i32;

    fn bound(&self) -> i32 {
        i32::MAX
    }

    fn value(&self, board: &ChessBoard, length: u32) -> Self::V {
        if board.is_done() {
            return SolverHeuristic.value(board, length);
        };

        let mut total = 0;

        for piece in ALL_PIECES {
            let value = match piece {
                Piece::Pawn => 1,
                Piece::Knight | Piece::Bishop => 3,
                Piece::Rook => 5,
                Piece::Queen => 9,
                Piece::King => 0,
            };

            for square in *board.inner().pieces(piece) {
                let matches = board.inner().color_on(square).unwrap() == board.inner().side_to_move();
                let sign = if matches { 1 } else { -1 };

                total += value * sign;
            }
        }

        total
    }
}