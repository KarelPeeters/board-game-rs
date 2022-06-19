use std::fmt::{Debug, Display};
use std::ops::Sub;

use num_traits::One;
use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::util::coord::Coord;

/// The symmetry group associated with a Board. An instance of this group maps a board and moves such that everything
/// about the board and its state is invariant under this mapping.
/// The [Default] value is the identity element.
pub trait Symmetry: 'static + Default + Debug + Copy + Clone + Eq + PartialEq + Send + Sync {
    fn all() -> &'static [Self];
    fn inverse(self) -> Self;
    fn is_unit() -> bool {
        Self::all().len() == 1
    }
}

#[derive(Debug)]
pub struct SymmetryDistribution;

impl<S: Symmetry + Sized> Distribution<S> for SymmetryDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> S {
        *S::all().choose(rng).expect("A symmetry group cannot be empty")
    }
}

/// The trivial symmetry group with only the identity, can be used as a conservative implementation.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct UnitSymmetry;

impl Symmetry for UnitSymmetry {
    fn all() -> &'static [Self] {
        &[Self]
    }
    fn inverse(self) -> Self {
        Self
    }
}

impl Default for UnitSymmetry {
    fn default() -> Self {
        UnitSymmetry
    }
}

/// The D1 symmetry group, representing a single axis mirror, resulting in 2 elements.
///
/// The `Default::default()` value means no transformation.
///
/// The representation is such that first x and y are optionally transposed,
/// then each axis is optionally flipped separately.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct D1Symmetry {
    pub mirror: bool,
}

impl D1Symmetry {
    pub const fn new(mirror: bool) -> Self {
        D1Symmetry { mirror }
    }

    pub fn map_axis<V: Copy + Sub<Output = V> + One>(self, x: V, size: V) -> V {
        let max = size - V::one();
        if self.mirror {
            max - x
        } else {
            x
        }
    }
}

impl Symmetry for D1Symmetry {
    fn all() -> &'static [Self] {
        const ALL: [D1Symmetry; 2] = [D1Symmetry::new(false), D1Symmetry::new(true)];
        &ALL
    }

    fn inverse(self) -> Self {
        self
    }
}

/// The D4 symmetry group that can represent any combination of
/// flips, rotating and transposing, which result in 8 distinct elements.
///
/// The representation is such that first x and y are optionally transposed,
/// then each axis is optionally flipped separately.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct D4Symmetry {
    pub transpose: bool,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl D4Symmetry {
    pub const fn new(transpose: bool, flip_x: bool, flip_y: bool) -> Self {
        D4Symmetry {
            transpose,
            flip_x,
            flip_y,
        }
    }

    pub fn map_xy<V: Copy + Sub<Output = V> + One + Display>(self, mut x: V, mut y: V, size: V) -> (V, V) {
        println!("Mapping {} {} {}", x, y, size);

        let max = size - V::one();

        if self.transpose {
            std::mem::swap(&mut x, &mut y)
        };
        if self.flip_x {
            x = max - x
        };
        if self.flip_y {
            y = max - y
        };

        println!("    to {} {}", x, y);

        (x, y)
    }

    pub fn map_coord<const X: u8, const Y: u8>(self, coord: Coord<X, Y>, size: u8) -> Coord<X, Y> {
        assert!(size <= X && size <= Y);
        let (x, y) = self.map_xy(coord.x(), coord.y(), size);
        Coord::from_xy(x, y)
    }
}

impl Symmetry for D4Symmetry {
    fn all() -> &'static [Self] {
        const ALL: [D4Symmetry; 8] = [
            D4Symmetry::new(false, false, false),
            D4Symmetry::new(false, false, true),
            D4Symmetry::new(false, true, false),
            D4Symmetry::new(false, true, true),
            D4Symmetry::new(true, false, false),
            D4Symmetry::new(true, false, true),
            D4Symmetry::new(true, true, false),
            D4Symmetry::new(true, true, true),
        ];
        &ALL
    }

    fn inverse(self) -> Self {
        D4Symmetry::new(
            self.transpose,
            if self.transpose { self.flip_y } else { self.flip_x },
            if self.transpose { self.flip_x } else { self.flip_y },
        )
    }
}
