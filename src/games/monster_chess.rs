use monster_chess::board::Board as NativeBoard;

use crate::{board::{Board, Player, BoardMoves, BoardSymmetry}, impl_unit_symmetry_board, symmetry::UnitSymmetry};

pub struct MonsterBoard<'a, const T: usize>(pub NativeBoard<'a, T>);

// Couldn't use the `impl_unit_symmetry_board` because of MonsterBoard's generics.
impl<'a, const T: usize> BoardSymmetry<MonsterBoard<'a, T>> for MonsterBoard<'a, T> {
    type Symmetry = UnitSymmetry;
    type CanonicalKey = ();

    fn map(&self, _: Self::Symmetry) -> Self {
        self.clone()
    }

    fn map_move(
        &self,
        _: Self::Symmetry,
        mv: <MonsterBoard<'a, T> as Board>::Move,
    ) -> <MonsterBoard<'a, T> as Board>::Move {
        mv
    }

    fn canonical_key(&self) -> Self::CanonicalKey {}
}

impl<'a, const T: usize> BoardMoves<'a, MonsterBoard<'a, T>> for MonsterBoard<'a, T> {
     
}

impl<'a, const T: usize> Board for MonsterBoard<'a, T> {
    fn next_player(&self) -> Player {
        match self.0.state.moving_team {
            0 => Player::A,
            1 => Player::B
        }
    }
}