//! Board setup: the two hardcoded default layouts, and d12-based random
//! army generation per the design doc's front/middle/back rank tables.

use crate::board::{Board, Pos, HEIGHT, WIDTH};
use crate::piece::{Piece, PieceType, Player};

/// A small, dependency-free xorshift64 PRNG used only for `random_setup`.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform roll of a d12: returns 1..=12.
    pub fn roll_d12(&mut self) -> u8 {
        (self.next_u64() % 12) as u8 + 1
    }
}

// ---------- default setups ----------

fn parse_rank(layout: &str) -> Vec<PieceType> {
    layout
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            'P' => PieceType::Pawn,
            'N' => PieceType::Knight,
            'B' => PieceType::Bishop,
            'F' => PieceType::Falcon,
            // The design doc's layout grids use 'E' for Elephant/Mammoth.
            'E' | 'M' => PieceType::Mammoth,
            'R' => PieceType::Rook,
            'Q' => PieceType::Queen,
            'K' => PieceType::King,
            other => panic!("unknown piece code '{}' in rank layout", other),
        })
        .collect()
}

fn place_rank(board: &mut Board, player: Player, row: i8, layout: &str) {
    for (col, kind) in parse_rank(layout).into_iter().enumerate() {
        board.place_new(Pos::new(col as i8, row), Piece { player, kind });
    }
}

/// Default-A, "Guinevere": asymmetric armies (Black 74 points, White 80
/// points per the design doc). Black's rooks start on the board's true
/// corners; White's do not (Queens occupy those corners instead), so only
/// Black can ever castle under this setup, by the letter of the castling
/// rule ("a rook that started in a corner square").
pub fn default_setup_a() -> Board {
    let mut board = Board::empty();
    place_rank(&mut board, Player::Black, 0, "R E F K F E R");
    place_rank(&mut board, Player::Black, 1, "N B B Q B B N");
    place_rank(&mut board, Player::Black, 2, "P P P P P P P");
    place_rank(&mut board, Player::White, HEIGHT - 3, "P P P P P P P");
    place_rank(&mut board, Player::White, HEIGHT - 2, "N B F N B F N");
    place_rank(&mut board, Player::White, HEIGHT - 1, "Q R E K E R Q");
    board
}

/// Default-B: symmetric armies, both sides' rooks start on true corners,
/// so castling is available to both. This is arbor's chosen opening
/// position for ChessBattle70 (see `Game::new`).
pub fn default_setup_b() -> Board {
    let mut board = Board::empty();
    place_rank(&mut board, Player::Black, 0, "R E Q K Q E R");
    place_rank(&mut board, Player::Black, 1, "B B N F N B B");
    place_rank(&mut board, Player::Black, 2, "P P P P P P P");
    place_rank(&mut board, Player::White, HEIGHT - 3, "P P P P P P P");
    place_rank(&mut board, Player::White, HEIGHT - 2, "B B N F N B B");
    place_rank(&mut board, Player::White, HEIGHT - 1, "R E Q K Q E R");
    board
}

// ---------- random army generation ----------

fn front_rank_piece(roll: u8) -> PieceType {
    match roll {
        1 | 2 => PieceType::Falcon,
        3 | 4 => PieceType::Bishop,
        5 | 6 => PieceType::Knight,
        _ => PieceType::Pawn, // 7-12
    }
}

fn middle_rank_piece(roll: u8) -> PieceType {
    match roll {
        1 => PieceType::Pawn,
        2 | 3 => PieceType::Mammoth,
        4 | 5 => PieceType::Rook,
        6 | 7 => PieceType::Bishop,
        8 | 9 => PieceType::Falcon,
        10 | 11 => PieceType::Knight,
        _ => PieceType::Queen, // 12
    }
}

fn back_rank_piece(roll: u8) -> PieceType {
    match roll {
        1..=3 => PieceType::Queen,
        4..=6 => PieceType::Mammoth,
        7..=9 => PieceType::Rook,
        10 => PieceType::Bishop,
        11 => PieceType::Falcon,
        _ => PieceType::Knight, // 12
    }
}

/// Fills one player's three ranks. King is placed dead-center of the back
/// rank without a roll (per the design doc); the other 6 back-rank squares
/// each get their own independent d12 roll.
fn random_army(board: &mut Board, player: Player, rng: &mut Rng, back_row: i8, middle_row: i8, front_row: i8) {
    let king_col = WIDTH / 2;

    for col in 0..WIDTH {
        let kind = if col == king_col {
            PieceType::King
        } else {
            back_rank_piece(rng.roll_d12())
        };
        board.place_new(Pos::new(col, back_row), Piece { player, kind });
    }
    for col in 0..WIDTH {
        let kind = middle_rank_piece(rng.roll_d12());
        board.place_new(Pos::new(col, middle_row), Piece { player, kind });
    }
    for col in 0..WIDTH {
        let kind = front_rank_piece(rng.roll_d12());
        board.place_new(Pos::new(col, front_row), Piece { player, kind });
    }
}

/// A fully random army for both sides, independently rolled (not mirrored
/// or symmetric -- matches the design doc's "box of chessmen and a d12"
/// framing). Not currently wired into `Game::new`/the UI, but exercised by
/// this crate's own tests.
#[allow(dead_code)]
pub fn random_setup(seed: u64) -> Board {
    let mut board = Board::empty();
    let mut rng = Rng::new(seed);
    random_army(&mut board, Player::Black, &mut rng, 0, 1, 2);
    random_army(&mut board, Player::White, &mut rng, HEIGHT - 1, HEIGHT - 2, HEIGHT - 3);
    board
}
