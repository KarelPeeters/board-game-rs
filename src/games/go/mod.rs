pub use board::*;
pub use chains::*;
pub use hash::*;
pub use io::*;
pub use link::*;
pub use rules::*;
pub use tile::*;

// TODO bump to true limit of 256?
//  maybe leave _some_ room for edge case stuff?
//  ensure unit test checking code uses u64 for counting to make sure
pub const GO_MAX_SIZE: u8 = 25;
pub const GO_MAX_AREA: u16 = GO_MAX_SIZE as u16 * GO_MAX_SIZE as u16;

mod board;
mod chains;
mod hash;
mod io;
mod link;
mod rules;
mod tile;
