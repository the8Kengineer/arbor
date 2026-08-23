//! **ChessBattle70**: chess on a 7-file x 10-rank board with two extra
//! piece types (Falcon and Mammoth) and configurable movement rules for
//! them, implementing arbor's `GameState` trait.
//!
//! - Board/piece representation for the 7x10 grid.
//! - Full legal move generation (`check::legal_moves`) for all 8 piece
//!   types, including the Falcon (16-square forced-3-step leaper) and
//!   Mammoth (trampling rook) under the default ruleset, plus
//!   Seirawan-Harper and "Realism" alternates via `RaptorPachydermRules`.
//! - Castling (King must be unmoved and not currently/passing-through
//!   check; the participating Rook must have started the game on a
//!   physical board corner), en passant, and auto-promotion to Queen.
//! - Check / checkmate / stalemate detection (`check` module).
//! - Default-A ("Guinevere", asymmetric) and Default-B (symmetric, arbor's
//!   chosen opening position - see `Game::new`) setups, plus d12-weighted
//!   random army generation, all from the design doc.

pub mod board;
pub mod check;
pub mod game;
pub mod movegen;
pub mod moves;
pub mod piece;
pub mod rules;
pub mod setup;
#[cfg(test)]
mod test;

pub use board::{Board, Pos, HEIGHT, WIDTH};
pub use game::Game;
pub use moves::{Move, SpecialMove};
pub use piece::{Piece, PieceType, Player};
pub use rules::RaptorPachydermRules;
