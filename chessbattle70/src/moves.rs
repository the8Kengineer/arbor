//! Move representation.

use crate::board::Pos;
use crate::piece::Piece;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialMove {
    None,
    DoublePawnStep,
    EnPassant {
        captured_pawn: Pos,
    },
    /// Both the king's and rook's destinations are carried on the move
    /// itself (rather than re-derived at apply time) to keep
    /// `Game::make_move` simple and hard to get wrong.
    Castle {
        rook_from: Pos,
        rook_to: Pos,
    },
}

/// `PartialEq` is required by arbor's `Action` trait (used e.g. by
/// `MCTS::advance` to match a played action against a tree's children).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    pub from: Pos,
    pub to: Pos,
    /// The piece as it will be *after* landing on `to` -- almost always
    /// unchanged from what was on `from`, except a pawn reaching the far
    /// rank is promoted to a Queen here (see `movegen::pawn_moves`).
    pub piece: Piece,
    /// The captured piece and *its own* square -- usually equal to `to`,
    /// but differs for en passant (captured pawn isn't on `to`) and for
    /// Mammoth trample moves (captured piece is short of `to`).
    pub captured: Option<(Pos, Piece)>,
    pub special: SpecialMove,
}
