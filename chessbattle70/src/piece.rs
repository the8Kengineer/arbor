//! Piece and player primitives.

/// The eight piece types, numbered 1-8 to match the design doc (used for
/// the random-army d12 tables and for serialization elsewhere).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    /// Slot 4. Physically the same piece as a Seirawan-Harper "Hawk" --
    /// which ruleset governs its movement is chosen via `RaptorPachydermRules`.
    Falcon,
    /// Slot 5. Physically the same piece as a Seirawan-Harper "Elephant".
    Mammoth,
    Rook,
    Queen,
    King,
}

impl PieceType {
    /// Single-letter code used for setup layouts and debug rendering.
    pub fn code(self) -> char {
        match self {
            PieceType::Pawn => 'P',
            PieceType::Knight => 'N',
            PieceType::Bishop => 'B',
            PieceType::Falcon => 'F',
            PieceType::Mammoth => 'M',
            PieceType::Rook => 'R',
            PieceType::Queen => 'Q',
            PieceType::King => 'K',
        }
    }

    /// 1-8 numbering from the design doc.
    pub fn number(self) -> u8 {
        match self {
            PieceType::Pawn => 1,
            PieceType::Knight => 2,
            PieceType::Bishop => 3,
            PieceType::Falcon => 4,
            PieceType::Mammoth => 5,
            PieceType::Rook => 6,
            PieceType::Queen => 7,
            PieceType::King => 8,
        }
    }

    /// Nominal point value (per the design doc's FalconMammoth-ruleset table),
    /// used by `Game::custom_evaluation`'s material heuristic. The King's
    /// value (2) is nominal only -- it can't actually be traded away; check/
    /// checkmate ends the game before that would matter.
    pub fn value(self) -> u8 {
        match self {
            PieceType::Pawn => 1,
            PieceType::Knight => 3,
            PieceType::Bishop => 3,
            PieceType::Rook => 5,
            PieceType::Falcon => 6,
            PieceType::Mammoth => 7,
            PieceType::Queen => 9,
            PieceType::King => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::White => Player::Black,
            Player::Black => Player::White,
        }
    }

    /// Row-direction (added to a pawn's row) this player's pawns advance.
    /// White starts near the high-numbered rows and advances toward row 0;
    /// Black starts near row 0 and advances toward the high-numbered rows.
    pub fn forward(self) -> i8 {
        match self {
            Player::White => -1,
            Player::Black => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub player: Player,
    pub kind: PieceType,
}
