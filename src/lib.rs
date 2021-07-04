/// The chess logic
/// Intended to be a seperatable lib (and support interop with other langs later).

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod player;
mod square;

pub mod game;
pub mod proto;
pub mod replay;

pub use player::Player;
pub use square::Square;
