//! Check / checkmate / stalemate detection, built on top of movegen's
//! pseudo-legal move generation.

use crate::board::{Board, Pos};
use crate::movegen::pseudo_legal_moves_for;
use crate::moves::{Move, SpecialMove};
use crate::piece::{PieceType, Player};
use crate::rules::RaptorPachydermRules;

/// True if any of `by`'s pieces could, right now, move onto `target`.
///
/// The King's own contribution is checked directly via plain adjacency
/// (rather than by calling its full move generator) specifically to avoid
/// infinite recursion: `movegen::king_moves`'s castling logic itself calls
/// this function (to enforce "cannot castle out of/through check"), so
/// routing a King's *attack* contribution back through that same castling
/// logic would recurse forever whenever either king is still eligible to
/// castle.
pub fn square_is_attacked(board: &Board, target: Pos, by: Player, rules: RaptorPachydermRules) -> bool {
    for (pos, piece) in board.all_pieces() {
        if piece.player != by {
            continue;
        }
        if piece.kind == PieceType::King {
            let dc = (target.col - pos.col).abs();
            let dr = (target.row - pos.row).abs();
            if dc <= 1 && dr <= 1 && (dc, dr) != (0, 0) {
                return true;
            }
            continue;
        }
        if pseudo_legal_moves_for(board, pos, rules).iter().any(|m| m.to == target) {
            return true;
        }
    }
    false
}

fn king_pos(board: &Board, player: Player) -> Option<Pos> {
    board
        .all_pieces()
        .into_iter()
        .find(|(_, piece)| piece.player == player && piece.kind == PieceType::King)
        .map(|(pos, _)| pos)
}

/// True if `player`'s king is currently attacked. Returns `false` if the
/// king isn't on the board (shouldn't happen - checkmate ends the game
/// before a king could be captured - but avoids a panic either way).
pub fn king_in_check(board: &Board, player: Player, rules: RaptorPachydermRules) -> bool {
    match king_pos(board, player) {
        Some(pos) => square_is_attacked(board, pos, player.opponent(), rules),
        None => false,
    }
}

/// Applies `mv` to a scratch copy of `board` (mirroring the piece-placement
/// half of `Game::make_move`; turn/meta bookkeeping is irrelevant to a
/// one-shot check test and is skipped).
fn apply(board: &Board, mv: Move) -> Board {
    let mut next = *board;
    if let Some((cap_pos, _)) = mv.captured {
        next.set(cap_pos, None);
    }
    next.set(mv.from, None);
    next.set(mv.to, Some(mv.piece));
    if let SpecialMove::Castle { rook_from, rook_to } = mv.special {
        if let Some(rook) = next.get(rook_from) {
            next.set(rook_from, None);
            next.set(rook_to, Some(rook));
        }
    }
    next
}

/// Every pseudo-legal move for `player` that doesn't leave their own king
/// in check afterward -- i.e. the truly legal moves. This also correctly
/// forbids a king from capturing a *defended* enemy piece "for free": the
/// simulated post-move board still has the defending piece on it, so it
/// still attacks the capture square.
pub fn legal_moves(board: &Board, player: Player, rules: RaptorPachydermRules) -> Vec<Move> {
    let mut out = Vec::new();
    for (pos, piece) in board.all_pieces() {
        if piece.player != player {
            continue;
        }
        for mv in pseudo_legal_moves_for(board, pos, rules) {
            let after = apply(board, mv);
            if !king_in_check(&after, player, rules) {
                out.push(mv);
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    InPlay,
    /// `player` (the side to move) has no legal moves and is in check.
    Checkmate(Player),
    /// `player` (the side to move) has no legal moves and is not in check.
    Stalemate(Player),
}

pub fn game_status(board: &Board, player: Player, rules: RaptorPachydermRules) -> GameStatus {
    if !legal_moves(board, player, rules).is_empty() {
        return GameStatus::InPlay;
    }
    if king_in_check(board, player, rules) {
        GameStatus::Checkmate(player)
    } else {
        GameStatus::Stalemate(player)
    }
}
