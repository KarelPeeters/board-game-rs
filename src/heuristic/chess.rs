use std::cmp::{max, Ordering};

use cozy_chess::{Color, Piece};

use crate::ai::minimax::Heuristic;
use crate::ai::solver::SolverHeuristic;
use crate::board::Board;
use crate::games::chess::ChessBoard;

#[derive(Debug)]
pub struct ChessPieceValueHeuristic;

impl Heuristic<ChessBoard> for ChessPieceValueHeuristic {
    type V = i32;

    fn value(&self, board: &ChessBoard, length: u32) -> Self::V {
        if board.is_done() {
            return SolverHeuristic.value(board, length).to_i32();
        }

        let mut total = 0;

        for piece in Piece::ALL {
            let value = match piece {
                Piece::Pawn => 1,
                Piece::Knight | Piece::Bishop => 3,
                Piece::Rook => 5,
                Piece::Queen => 9,
                Piece::King => 0,
            };

            for color in Color::ALL {
                let sign = if color == board.inner().side_to_move() { 1 } else { -1 };
                let count = board.inner().colored_pieces(color, piece).len();
                total += sign * value * (count as i32);
            }
        }

        total
    }

    fn merge(old: Self::V, new: Self::V) -> (Self::V, Ordering) {
        (max(old, new), new.cmp(&old))
    }
}
