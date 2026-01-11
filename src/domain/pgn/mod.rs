//! PGN (Portable Game Notation) parsing and writing.
//!
//! Uses the `pgn-reader` crate for fast, streaming PGN parsing.

mod types;
mod visitor;

pub use types::*;
pub use visitor::{load_pgn_file, parse_pgn};
