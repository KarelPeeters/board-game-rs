use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoroshiro64StarStar;

pub fn consistent_rng() -> impl Rng {
    Xoroshiro64StarStar::seed_from_u64(0)
}
