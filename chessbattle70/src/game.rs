//! Ties board + turn state + move generation/application together, and
//! implements arbor's `GameState` trait.

use std::fmt;
use std::fmt::Display;

use arbor::{GameResult, GameState};

use crate::board::{Board, Pos, HEIGHT, WIDTH};
use crate::check;
use crate::moves::{Move, SpecialMove};
use crate::piece::Player;
use crate::rules::RaptorPachydermRules;
use crate::setup;

/// `Copy` is required by arbor's `GameState` trait -- MCTS clones game
/// states constantly, at every node of every simulation, so this deliberately
/// carries no move history or other heap-allocated bookkeeping. En passant
/// and castling both key off per-square `SquareMeta` on `Board`, not replay.
#[derive(Debug, Clone, Copy)]
pub struct Game {
    pub board: Board,
    pub current_player: Player,
    pub rules: RaptorPachydermRules,
}

impl Game {
    /// arbor's opening position for ChessBattle70: Default-B (symmetric
    /// armies, both sides can castle) under the default FalconMammoth rules.
    pub fn new() -> Self {
        Game::from_setup(setup::default_setup_b(), RaptorPachydermRules::default())
    }

    pub fn from_setup(board: Board, rules: RaptorPachydermRules) -> Self {
        Game { board, current_player: Player::White, rules }
    }

    /// Pseudo-legal moves for the single piece at `pos` (empty if there's
    /// no piece there). Unlike `GameState::actions`, this does NOT filter
    /// out moves that would leave the mover's own king in check - handy
    /// for tests that want to inspect one piece's raw movement pattern in
    /// isolation. See `check::legal_moves` for the fully-filtered version.
    pub fn moves_for(&self, pos: Pos) -> Vec<Move> {
        crate::movegen::pseudo_legal_moves_for(&self.board, pos, self.rules)
    }

    /// Applies a move (as produced by `arbor::GameState::actions`) and
    /// advances the turn. Does not itself re-validate legality.
    pub fn make_move(&mut self, mv: Move) {
        // "Just double stepped" only matters for the very next move.
        for c in 0..WIDTH {
            for r in 0..HEIGHT {
                let p = Pos::new(c, r);
                let mut m = self.board.meta(p);
                if m.just_double_stepped {
                    m.just_double_stepped = false;
                    self.board.set_meta(p, m);
                }
            }
        }

        if let Some((cap_pos, _)) = mv.captured {
            self.board.set(cap_pos, None);
        }

        let mut moved_meta = self.board.meta(mv.from);
        moved_meta.has_moved = true;
        moved_meta.just_double_stepped = matches!(mv.special, SpecialMove::DoublePawnStep);

        self.board.set(mv.from, None);
        self.board.set(mv.to, Some(mv.piece));
        self.board.set_meta(mv.to, moved_meta);

        if let SpecialMove::Castle { rook_from, rook_to } = mv.special {
            if let Some(rook) = self.board.get(rook_from) {
                let mut rook_meta = self.board.meta(rook_from);
                rook_meta.has_moved = true;
                self.board.set(rook_from, None);
                self.board.set(rook_to, Some(rook));
                self.board.set_meta(rook_to, rook_meta);
            }
        }

        self.current_player = self.current_player.opponent();
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?} to move", self.current_player)?;
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                let ch = match self.board.get(Pos::new(col, row)) {
                    None => '.',
                    Some(piece) => {
                        let c = piece.kind.code();
                        if piece.player == Player::White { c } else { c.to_ascii_lowercase() }
                    }
                };
                write!(f, "{} ", ch)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl arbor::Player for Player {}
impl arbor::Action for Move {}

impl GameState<Player, Move> for Game {
    fn actions<F>(&self, f: &mut F) where F: FnMut(Move) {
        debug_assert!(self.gameover().is_none());
        for mv in check::legal_moves(&self.board, self.current_player, self.rules) {
            f(mv);
        }
    }

    fn make(&self, action: Move) -> Self {
        let mut next = *self;
        next.make_move(action);
        next
    }

    fn gameover(&self) -> Option<GameResult> {
        match check::game_status(&self.board, self.current_player, self.rules) {
            check::GameStatus::InPlay => None,
            // Mirrors the "side to play last always wins" pattern used
            // elsewhere in this workspace: the side about to move
            // (self.current_player) has no way out, so they've lost.
            check::GameStatus::Checkmate(_) => Some(GameResult::Lose),
            check::GameStatus::Stalemate(_) => Some(GameResult::Draw),
        }
    }

    fn player(&self) -> Player {
        self.current_player
    }

    /// Material-count heuristic (win probability for `self.player()`),
    /// squashed into (0,1) via tanh. Plain random-rollout-to-terminal (what
    /// every other arbor game uses) isn't practical here: legal move
    /// generation now does real work (simulate-and-check per candidate),
    /// and uniformly-random legal play on a board this size can run very
    /// long before naturally reaching checkmate/stalemate. This replaces a
    /// full playout with one static evaluation per leaf - use
    /// `.with_custom_evaluation()` to enable it.
    fn custom_evaluation(&self) -> f32 {
        let mine = self.board.total_value(self.current_player) as f32;
        let theirs = self.board.total_value(self.current_player.opponent()) as f32;
        0.5 + 0.5 * ((mine - theirs) / 10.0).tanh()
    }
}
