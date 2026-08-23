//! Pseudo-legal move generation, per piece type.
//!
//! "Pseudo-legal" means these moves obey each piece's own movement rules
//! and normal capture/blocking, but do not by themselves guarantee the
//! mover's own king isn't left capturable -- `check::legal_moves` filters
//! this module's output down to truly legal moves by simulating each one.

use crate::board::{Board, Pos, WIDTH};
use crate::check::square_is_attacked;
use crate::moves::{Move, SpecialMove};
use crate::piece::{Piece, PieceType, Player};
use crate::rules::RaptorPachydermRules;

const ROOK_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
const QUEEN_DIRS: [(i8, i8); 8] = [
    (1, 0), (-1, 0), (0, 1), (0, -1),
    (1, 1), (1, -1), (-1, 1), (-1, -1),
];
const KNIGHT_OFFSETS: [(i8, i8); 8] = [
    (1, 2), (2, 1), (2, -1), (1, -2),
    (-1, -2), (-2, -1), (-2, 1), (-1, 2),
];
const KING_OFFSETS: [(i8, i8); 8] = [
    (1, 0), (-1, 0), (0, 1), (0, -1),
    (1, 1), (1, -1), (-1, 1), (-1, -1),
];

/// The Falcon's 16 reachable squares: every square at Manhattan distance 1
/// or 3 (the only distances reachable via exactly three orthogonal unit
/// steps, direction chosen freely -- including repeats -- at each step).
/// Because any square in this set is reachable by *some* 3-step path
/// entirely of the mover's choosing, and only the landing square's
/// occupancy matters ("airborne" for the first two steps), the Falcon can
/// be treated as a fixed-offset leaper, exactly like a Knight.
const FALCON_OFFSETS: [(i8, i8); 16] = [
    (1, 0), (-1, 0), (0, 1), (0, -1),
    (0, 3), (0, -3), (3, 0), (-3, 0),
    (1, 2), (1, -2), (-1, 2), (-1, -2),
    (2, 1), (2, -1), (-2, 1), (-2, -1),
];

pub fn pseudo_legal_moves_for(board: &Board, from: Pos, rules: RaptorPachydermRules) -> Vec<Move> {
    let piece = match board.get(from) {
        Some(p) => p,
        None => return Vec::new(),
    };
    match piece.kind {
        PieceType::Pawn => pawn_moves(board, from, piece.player),
        PieceType::Knight => leap_moves(board, from, piece.player, &KNIGHT_OFFSETS),
        PieceType::Bishop => slide_moves(board, from, piece.player, &BISHOP_DIRS),
        PieceType::Rook => slide_moves(board, from, piece.player, &ROOK_DIRS),
        PieceType::Queen => slide_moves(board, from, piece.player, &QUEEN_DIRS),
        PieceType::King => king_moves(board, from, piece.player, rules),
        PieceType::Falcon => falcon_moves(board, from, piece.player, rules),
        PieceType::Mammoth => mammoth_moves(board, from, piece.player, rules),
    }
}

// ---------- shared movement primitives ----------

fn slide_moves(board: &Board, from: Pos, player: Player, dirs: &[(i8, i8)]) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = Vec::new();
    for &(dc, dr) in dirs {
        let mut cur = from;
        loop {
            cur = cur.offset(dc, dr);
            if !cur.in_bounds() || board.is_friendly(cur, player) {
                break;
            }
            let captured = board.get(cur).map(|p| (cur, p));
            let stop = captured.is_some();
            moves.push(Move { from, to: cur, piece, captured, special: SpecialMove::None });
            if stop {
                break;
            }
        }
    }
    moves
}

fn leap_moves(board: &Board, from: Pos, player: Player, offsets: &[(i8, i8)]) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = Vec::new();
    for &(dc, dr) in offsets {
        let to = from.offset(dc, dr);
        if !to.in_bounds() || board.is_friendly(to, player) {
            continue;
        }
        let captured = board.get(to).map(|p| (to, p));
        moves.push(Move { from, to, piece, captured, special: SpecialMove::None });
    }
    moves
}

/// Bishop-shaped moves that ignore every intervening piece entirely (used
/// by the Hawk's diagonal component in the Realism variant).
fn fly_diagonal_moves(board: &Board, from: Pos, player: Player) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = Vec::new();
    for &(dc, dr) in &BISHOP_DIRS {
        let mut cur = from;
        loop {
            cur = cur.offset(dc, dr);
            if !cur.in_bounds() {
                break;
            }
            if !board.is_friendly(cur, player) {
                let captured = board.get(cur).map(|p| (cur, p));
                moves.push(Move { from, to: cur, piece, captured, special: SpecialMove::None });
            }
            // no break here -- it flies over friendly and enemy pieces alike
        }
    }
    moves
}

/// Knight-shaped moves that require a clear orthogonal L-path (used by the
/// Elephant in the Realism variant). Either of the two possible L-routes
/// being fully clear is enough to permit the move.
fn clear_path_knight_moves(board: &Board, from: Pos, player: Player) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = Vec::new();
    for &(dc, dr) in &KNIGHT_OFFSETS {
        let to = from.offset(dc, dr);
        if !to.in_bounds() || board.is_friendly(to, player) {
            continue;
        }

        let (long_step, short_step) = if dc.abs() == 2 {
            ((dc.signum(), 0), (0, dr.signum()))
        } else {
            ((0, dr.signum()), (dc.signum(), 0))
        };

        // Route 1: travel the long (2-square) axis first, then the short axis.
        let r1_mid = from.offset(long_step.0, long_step.1);
        let r1_elbow = r1_mid.offset(long_step.0, long_step.1);
        let route1_clear = board.is_empty(r1_mid) && board.is_empty(r1_elbow);

        // Route 2: travel the short (1-square) axis first, then the long axis.
        let r2_elbow = from.offset(short_step.0, short_step.1);
        let r2_mid = r2_elbow.offset(long_step.0, long_step.1);
        let route2_clear = board.is_empty(r2_elbow) && board.is_empty(r2_mid);

        if route1_clear || route2_clear {
            let captured = board.get(to).map(|p| (to, p));
            moves.push(Move { from, to, piece, captured, special: SpecialMove::None });
        }
    }
    moves
}

// ---------- pawn ----------

/// The far rank a `player`'s pawn promotes on reaching.
fn promotion_row(player: Player) -> i8 {
    match player {
        Player::White => 0,
        Player::Black => crate::board::HEIGHT - 1,
    }
}

/// The piece that lands on `to` after this pawn move -- a Queen if `to` is
/// the far rank (auto-promotion, no promotion-choice UI for v1), otherwise
/// the pawn itself.
fn landing_piece(player: Player, to: Pos) -> Piece {
    if to.row == promotion_row(player) {
        Piece { player, kind: PieceType::Queen }
    } else {
        Piece { player, kind: PieceType::Pawn }
    }
}

fn pawn_moves(board: &Board, from: Pos, player: Player) -> Vec<Move> {
    let mut moves = Vec::new();
    let fwd = player.forward();

    let one = from.offset(0, fwd);
    if one.in_bounds() && board.is_empty(one) {
        moves.push(Move { from, to: one, piece: landing_piece(player, one), captured: None, special: SpecialMove::None });
        let two = from.offset(0, fwd * 2);
        if !board.meta(from).has_moved && two.in_bounds() && board.is_empty(two) {
            moves.push(Move { from, to: two, piece: landing_piece(player, two), captured: None, special: SpecialMove::DoublePawnStep });
        }
    }

    for dc in [-1i8, 1i8] {
        let cap = from.offset(dc, fwd);
        if !cap.in_bounds() {
            continue;
        }
        if board.is_enemy(cap, player) {
            let captured = board.get(cap).map(|p| (cap, p));
            moves.push(Move { from, to: cap, piece: landing_piece(player, cap), captured, special: SpecialMove::None });
        } else if board.is_empty(cap) {
            // En passant: the square beside `from` (same row) holds an
            // enemy pawn that just double-stepped.
            let adj = Pos::new(cap.col, from.row);
            if let Some(adj_piece) = board.get(adj) {
                if adj_piece.player != player
                    && adj_piece.kind == PieceType::Pawn
                    && board.meta(adj).just_double_stepped
                {
                    moves.push(Move {
                        from,
                        to: cap,
                        piece: landing_piece(player, cap),
                        captured: Some((adj, adj_piece)),
                        special: SpecialMove::EnPassant { captured_pawn: adj },
                    });
                }
            }
        }
    }

    moves
}

// ---------- king (incl. castling) ----------

fn king_moves(board: &Board, from: Pos, player: Player, rules: RaptorPachydermRules) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = leap_moves(board, from, player, &KING_OFFSETS);

    if board.meta(from).has_moved {
        return moves;
    }

    // Cannot castle out of check.
    if square_is_attacked(board, from, player.opponent(), rules) {
        return moves;
    }

    for (rook_pos, rook) in board.all_pieces() {
        if rook.player != player || rook.kind != PieceType::Rook {
            continue;
        }
        if rook_pos.row != from.row {
            continue;
        }
        let rook_meta = board.meta(rook_pos);
        if rook_meta.has_moved || rook_meta.start_pos != Some(rook_pos) || !rook_pos.is_corner() {
            continue;
        }

        let dir: i8 = if rook_pos.col > from.col { 1 } else { -1 };

        // Squares strictly between king and rook must be empty.
        let mut c = from.col + dir;
        let mut clear = true;
        while c != rook_pos.col {
            if !board.is_empty(Pos::new(c, from.row)) {
                clear = false;
                break;
            }
            c += dir;
        }
        if !clear {
            continue;
        }

        let king_to = Pos::new(from.col + dir * 2, from.row);
        let rook_to = Pos::new(from.col + dir, from.row);
        if !king_to.in_bounds() || !(0..WIDTH).contains(&rook_to.col) {
            continue;
        }

        // Cannot castle through an attacked square (the king's transit
        // square, i.e. rook_to). Landing in check is filtered out later by
        // check::legal_moves' general simulate-and-check pass.
        if square_is_attacked(board, rook_to, player.opponent(), rules) {
            continue;
        }

        moves.push(Move {
            from,
            to: king_to,
            piece,
            captured: None,
            special: SpecialMove::Castle { rook_from: rook_pos, rook_to },
        });
    }

    moves
}

// ---------- falcon ----------

fn falcon_moves(board: &Board, from: Pos, player: Player, rules: RaptorPachydermRules) -> Vec<Move> {
    match rules {
        RaptorPachydermRules::FalconMammoth => leap_moves(board, from, player, &FALCON_OFFSETS),
        RaptorPachydermRules::SeirawanHarper => {
            let mut m = leap_moves(board, from, player, &KNIGHT_OFFSETS);
            m.extend(slide_moves(board, from, player, &BISHOP_DIRS));
            m
        }
        RaptorPachydermRules::Realism => {
            // Knight-component leaps normally (a leap already ignores
            // blockers); the bishop-component additionally flies over
            // anything in its path.
            let mut m = leap_moves(board, from, player, &KNIGHT_OFFSETS);
            m.extend(fly_diagonal_moves(board, from, player));
            m
        }
    }
}

// ---------- mammoth ----------

fn mammoth_moves(board: &Board, from: Pos, player: Player, rules: RaptorPachydermRules) -> Vec<Move> {
    match rules {
        RaptorPachydermRules::FalconMammoth => mammoth_trample_moves(board, from, player),
        RaptorPachydermRules::SeirawanHarper => {
            let mut m = leap_moves(board, from, player, &KNIGHT_OFFSETS);
            m.extend(slide_moves(board, from, player, &ROOK_DIRS));
            m
        }
        RaptorPachydermRules::Realism => {
            let mut m = clear_path_knight_moves(board, from, player);
            m.extend(slide_moves(board, from, player, &ROOK_DIRS));
            m
        }
    }
}

/// Rook-like sliding, but on meeting the first enemy piece in a direction
/// the Mammoth gets both a normal capture-and-stop option AND (if the
/// squares beyond are empty) trample-through options landing further down
/// the same ray. Only the first piece hit can ever be captured in one
/// move; a second piece further down the ray still blocks the ray.
fn mammoth_trample_moves(board: &Board, from: Pos, player: Player) -> Vec<Move> {
    let piece = board.get(from).unwrap();
    let mut moves = Vec::new();
    for &(dc, dr) in &ROOK_DIRS {
        let mut cur = from;
        loop {
            cur = cur.offset(dc, dr);
            if !cur.in_bounds() {
                break;
            }
            match board.get(cur) {
                None => {
                    moves.push(Move { from, to: cur, piece, captured: None, special: SpecialMove::None });
                }
                Some(occupant) if occupant.player != player => {
                    // Normal capture, stopping on the captured square.
                    moves.push(Move {
                        from,
                        to: cur,
                        piece,
                        captured: Some((cur, occupant)),
                        special: SpecialMove::None,
                    });
                    // Trample: keep going past the captured piece onto any
                    // further empty squares along the same ray.
                    let mut trample = cur;
                    loop {
                        trample = trample.offset(dc, dr);
                        if !trample.in_bounds() || board.get(trample).is_some() {
                            break;
                        }
                        moves.push(Move {
                            from,
                            to: trample,
                            piece,
                            captured: Some((cur, occupant)),
                            special: SpecialMove::None,
                        });
                    }
                    break;
                }
                Some(_friendly) => break,
            }
        }
    }
    moves
}
