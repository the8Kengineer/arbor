//! The 7x10 board and per-square state needed for castling / en passant.

use crate::piece::{Piece, Player};

pub const WIDTH: i8 = 7;
pub const HEIGHT: i8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub col: i8,
    pub row: i8,
}

impl Pos {
    pub fn new(col: i8, row: i8) -> Self {
        Pos { col, row }
    }

    pub fn in_bounds(self) -> bool {
        self.col >= 0 && self.col < WIDTH && self.row >= 0 && self.row < HEIGHT
    }

    pub fn offset(self, dc: i8, dr: i8) -> Pos {
        Pos::new(self.col + dc, self.row + dr)
    }

    /// True for the four physical corners of the board. Castling requires
    /// the participating rook to have *started* the game on one of these.
    pub fn is_corner(self) -> bool {
        (self.col == 0 || self.col == WIDTH - 1) && (self.row == 0 || self.row == HEIGHT - 1)
    }
}

/// Per-square metadata needed for castling / double pawn steps / en
/// passant. Lives with the square (not the piece) so it resets cleanly
/// when a square becomes empty and is easy to look up by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SquareMeta {
    pub has_moved: bool,
    pub start_pos: Option<Pos>,
    /// Set for the one turn immediately after a pawn's two-square advance;
    /// `Game::make_move` clears this everywhere at the start of each move.
    pub just_double_stepped: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Board {
    squares: [[Option<Piece>; HEIGHT as usize]; WIDTH as usize],
    meta: [[SquareMeta; HEIGHT as usize]; WIDTH as usize],
}

impl Board {
    pub fn empty() -> Self {
        Board {
            squares: [[None; HEIGHT as usize]; WIDTH as usize],
            meta: [[SquareMeta::default(); HEIGHT as usize]; WIDTH as usize],
        }
    }

    pub fn get(&self, p: Pos) -> Option<Piece> {
        if !p.in_bounds() {
            return None;
        }
        self.squares[p.col as usize][p.row as usize]
    }

    pub fn meta(&self, p: Pos) -> SquareMeta {
        if !p.in_bounds() {
            return SquareMeta::default();
        }
        self.meta[p.col as usize][p.row as usize]
    }

    pub fn set(&mut self, p: Pos, piece: Option<Piece>) {
        self.squares[p.col as usize][p.row as usize] = piece;
    }

    pub fn set_meta(&mut self, p: Pos, meta: SquareMeta) {
        self.meta[p.col as usize][p.row as usize] = meta;
    }

    /// Place a piece as part of initial setup (records its start square).
    pub fn place_new(&mut self, p: Pos, piece: Piece) {
        self.set(p, Some(piece));
        self.set_meta(
            p,
            SquareMeta {
                has_moved: false,
                start_pos: Some(p),
                just_double_stepped: false,
            },
        );
    }

    pub fn is_empty(&self, p: Pos) -> bool {
        self.get(p).is_none()
    }

    pub fn is_enemy(&self, p: Pos, player: Player) -> bool {
        matches!(self.get(p), Some(piece) if piece.player != player)
    }

    pub fn is_friendly(&self, p: Pos, player: Player) -> bool {
        matches!(self.get(p), Some(piece) if piece.player == player)
    }

    pub fn all_pieces(&self) -> Vec<(Pos, Piece)> {
        let mut out = Vec::new();
        for c in 0..WIDTH {
            for r in 0..HEIGHT {
                let p = Pos::new(c, r);
                if let Some(piece) = self.get(p) {
                    out.push((p, piece));
                }
            }
        }
        out
    }

    /// Sum of `PieceType::value()` for one player's pieces currently on the
    /// board. Used by `Game::custom_evaluation`'s material heuristic.
    pub fn total_value(&self, player: Player) -> u32 {
        self.all_pieces()
            .into_iter()
            .filter(|(_, piece)| piece.player == player)
            .map(|(_, piece)| piece.kind.value() as u32)
            .sum()
    }
}
