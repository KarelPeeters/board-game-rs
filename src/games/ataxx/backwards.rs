use std::ops::ControlFlow;

use bitintr::Pdep;
use internal_iterator::InternalIterator;

use crate::board::Board;
use crate::games::ataxx::{coord_to_ring, AtaxxBoard, Move};
use crate::util::bitboard::BitBoard8;
use crate::util::coord::Coord8;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BackMove {
    pub mv: Move,
    pub converted: BitBoard8,
}

// TODO generalize backwards movegen to other games?
#[derive(Debug, Clone)]
pub struct BackMovesIterator<'a, B: Board> {
    board: &'a B,
}

impl AtaxxBoard {
    // TODO can this ever fail?
    pub fn back_moves(&self) -> BackMovesIterator<Self> {
        BackMovesIterator { board: self }
    }

    pub fn play_back(&mut self, mv: BackMove) {
        // TODO assert validness before mutating self
        // TODO ensure generated moves don't result in terminal boards

        let (them, us) = self.tiles_pov_mut();

        match mv.mv {
            Move::Pass => {
                // nothing extra to do
                debug_assert!(mv.converted.none());
            }
            Move::Copy { to } => {
                // remove to
                *us &= !BitBoard8::coord(to);

                // revert converted
                *us &= !mv.converted;
                *them |= mv.converted;
            }
            Move::Jump { from, to } => {
                // remove to, add from
                *us &= !BitBoard8::coord(to);
                *us |= BitBoard8::coord(from);

                // revert converted
                *us &= !mv.converted;
                *them |= mv.converted;
            }
        }

        self.next_player = self.next_player.other();
        self.moves_since_last_copy = 0;
        self.update_outcome();

        assert!(self.outcome().is_none());
    }
}

impl<'a> InternalIterator for BackMovesIterator<'a, AtaxxBoard> {
    type Item = BackMove;

    fn try_for_each<R, F>(self, mut f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        let board = self.board;
        let (them, us) = board.tiles_pov();

        let mut any = false;

        // "to" square cannot have any adjacent enemy tiles
        let potential_to = us & !them.adjacent();

        // singles
        for to in potential_to & us.adjacent() {
            any |= generate_captures(board, None, to, &mut f)?;
        }

        // doubles
        for to in potential_to & board.free_tiles().ring() {
            for from in coord_to_ring(to) & board.free_tiles() {
                any |= generate_captures(board, Some(from), to, &mut f)?;
            }
        }

        if !any {
            f(BackMove {
                mv: Move::Pass,
                converted: BitBoard8::EMPTY,
            })?;
        }

        ControlFlow::Continue(())
    }
}

fn generate_captures<R, F>(board: &AtaxxBoard, from: Option<Coord8>, to: Coord8, mut f: F) -> ControlFlow<R, bool>
where
    F: FnMut(BackMove) -> ControlFlow<R>,
{
    let (is_single, mv) = match from {
        Some(from) => (false, Move::Jump { from, to }),
        None => (true, Move::Copy { to }),
    };

    let (them, us) = board.tiles_pov();
    debug_assert_eq!(BitBoard8::coord(to).adjacent() & them, BitBoard8::EMPTY);

    // TODO add a lookup table for coord->adjacent?
    let potentially_converted = us & BitBoard8::coord(to).adjacent();

    // disallow single moves that convert everything, so skip the last full mask
    let limit_dense = if is_single {
        (1 << potentially_converted.count()) - 1
    } else {
        1 << potentially_converted.count()
    };

    let mut any = false;

    for converted_dense in 0..limit_dense {
        let converted = BitBoard8(converted_dense.pdep(potentially_converted.0));
        debug_assert_eq!(converted & !potentially_converted, BitBoard8::EMPTY);

        let back_mv = BackMove { mv, converted };
        f(back_mv)?;
        any = true;
    }

    ControlFlow::Continue(any)
}
