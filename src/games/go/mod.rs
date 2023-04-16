use static_assertions::const_assert;

pub use board::*;
pub use chains::*;
pub use hash::*;
pub use io::*;
pub use link::*;
pub use rules::*;
pub use tile::*;

include!(concat!(env!("OUT_DIR"), "/go_consts.rs"));

// leave some room for edge cases, in particular for future optimization in adjacent_in
const_assert!(GO_MAX_SIZE <= u8::MAX - 2);
// ensure there are some sentinel values available (currently only u16::MAX itself is used)
const_assert!(GO_MAX_AREA < u16::MAX - 8);

mod board;
mod chains;
mod hash;
mod io;
mod link;
mod rules;
mod stack_vec;
mod tile;
