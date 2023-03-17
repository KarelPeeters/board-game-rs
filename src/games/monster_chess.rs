use internal_iterator::InternalIterator;
use monster_chess::board::{Board as NativeBoard, actions::Action};
use std::fmt;
use std::fmt::Display;
use std::slice::Iter;

use crate::{board::{Board, Player, BoardMoves, BoardSymmetry}, impl_unit_symmetry_board, symmetry::UnitSymmetry};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MonsterBoard<'a, const T: usize>(pub NativeBoard<'a, T>);

impl<'a, const T: usize> Display for MonsterBoard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "{}", self.0);
    }
}

// Couldn't use the `impl_unit_symmetry_board` because of MonsterBoard's generics.
impl<const T: usize> BoardSymmetry<MonsterBoard<'static, T>> for MonsterBoard<'static, T> {
    type Symmetry = UnitSymmetry;
    type CanonicalKey = ();

    fn map(&self, _: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(
        &self,
        _: Self::Symmetry,
        mv: <MonsterBoard<'static, T> as Board>::Move,
    ) -> <MonsterBoard<'static, T> as Board>::Move {
        mv
    }

    fn canonical_key(&self) -> Self::CanonicalKey {}
}

impl<'a, const T: usize> BoardMoves<'a, MonsterBoard<'static, T>> for MonsterBoard<'static, T> {
    type AllMovesIterator = dyn Iterator<Item = Action>;
    type AvailableMovesIterator = dyn Iterator<Item = Action>;

    fn all_possible_moves() {
        
    }

    fn available_moves(&self) -> Result<Self::AvailableMovesIterator, crate::board::BoardDone> {
        self.0.generate_legal_moves(0)
    }
}

impl<'a, const T: usize> Board for MonsterBoard<'static, T> {
    
}