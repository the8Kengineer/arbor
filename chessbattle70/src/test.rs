use super::*;
use super::check;
use arbor::{GameResult, GameState, MCTS};

// ---------- ported from the original draft's engine_tests.rs ----------

#[test]
fn falcon_reaches_exactly_16_squares_when_unobstructed() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 5), Piece { player: Player::White, kind: PieceType::Falcon });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 5));
    assert_eq!(moves.len(), 16, "falcon should reach 16 squares on an open board");
}

#[test]
fn mammoth_can_capture_and_stop_or_trample_past() {
    let mut board = Board::empty();
    board.place_new(Pos::new(0, 5), Piece { player: Player::White, kind: PieceType::Mammoth });
    board.place_new(Pos::new(3, 5), Piece { player: Player::Black, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(0, 5));

    let rightward: Vec<_> = moves.iter().filter(|m| m.to.row == 5 && m.to.col > 0).collect();
    let destinations: Vec<i8> = rightward.iter().map(|m| m.to.col).collect();

    // (1,5) and (2,5) are plain empty-square moves; (3,5) is capture-and-
    // stop; (4,5),(5,5),(6,5) are trample-through landing options.
    assert!(destinations.contains(&1));
    assert!(destinations.contains(&2));
    assert!(destinations.contains(&3));
    assert!(destinations.contains(&4));
    assert!(destinations.contains(&5));
    assert!(destinations.contains(&6));
    assert_eq!(destinations.len(), 6);

    let capture_and_stop = rightward.iter().find(|m| m.to.col == 3).unwrap();
    assert_eq!(capture_and_stop.captured.unwrap().0, Pos::new(3, 5));

    let trample = rightward.iter().find(|m| m.to.col == 5).unwrap();
    assert_eq!(trample.captured.unwrap().0, Pos::new(3, 5), "trample moves still capture the piece at the pass-through square, not the landing square");
}

#[test]
fn mammoth_does_not_trample_a_second_piece() {
    let mut board = Board::empty();
    board.place_new(Pos::new(0, 5), Piece { player: Player::White, kind: PieceType::Mammoth });
    board.place_new(Pos::new(3, 5), Piece { player: Player::Black, kind: PieceType::Pawn });
    board.place_new(Pos::new(5, 5), Piece { player: Player::Black, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(0, 5));
    let destinations: Vec<i8> = moves.iter().filter(|m| m.to.row == 5 && m.to.col > 0).map(|m| m.to.col).collect();
    // trample can only reach col 4 (the empty square before the second
    // piece); it must not jump the second pawn or capture it.
    assert!(destinations.contains(&4));
    assert!(!destinations.contains(&5));
    assert!(!destinations.contains(&6));
}

// The design doc states Default-A's armies are worth 74 (Black) and 80
// (White) points. Computed against this crate's own PieceType::value()
// table (which prices the FalconMammoth ruleset's Mammoth slot at 7, per
// the doc's own "7 = Mammoth, Hawk" / "8 = Elephant" table on the same
// page), both totals come out 2 points lower: each army has two 'E'-slot
// pieces, and the doc's worked example appears to have priced them as
// Elephant (8) rather than Mammoth (7) - a self-inconsistency in the doc,
// not a code bug (confirmed: Default-B's two totals are still exactly
// equal either way, and the gap here is exactly 2 x (8-7) on both sides).
#[test]
fn default_setup_a_army_values() {
    let board = setup::default_setup_a();
    assert_eq!(board.total_value(Player::Black), 72);
    assert_eq!(board.total_value(Player::White), 78);
}

#[test]
fn default_setup_b_is_symmetric() {
    let board = setup::default_setup_b();
    assert_eq!(board.total_value(Player::Black), board.total_value(Player::White));
}

#[test]
fn castling_available_when_rook_started_on_a_corner_and_path_is_clear() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 9), Piece { player: Player::White, kind: PieceType::Rook });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 9));
    let castle = moves.iter().find(|m| matches!(m.special, SpecialMove::Castle { .. }));
    assert!(castle.is_some(), "expected a castling move toward the corner rook");
    let castle = castle.unwrap();
    assert_eq!(castle.to, Pos::new(1, 9));
    if let SpecialMove::Castle { rook_from, rook_to } = castle.special {
        assert_eq!(rook_from, Pos::new(0, 9));
        assert_eq!(rook_to, Pos::new(2, 9));
    }
}

#[test]
fn castling_unavailable_when_rook_did_not_start_on_a_corner() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    // Rook starts one square in from the corner -- shouldn't qualify.
    board.place_new(Pos::new(1, 9), Piece { player: Player::White, kind: PieceType::Rook });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 9));
    assert!(!moves.iter().any(|m| matches!(m.special, SpecialMove::Castle { .. })));
}

#[test]
fn en_passant_is_offered_immediately_after_a_double_step_and_not_after() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 2), Piece { player: Player::Black, kind: PieceType::Pawn });
    board.place_new(Pos::new(2, 4), Piece { player: Player::White, kind: PieceType::Pawn });
    let mut game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    game.current_player = Player::Black;

    let black_moves = game.moves_for(Pos::new(3, 2));
    let double_step = black_moves
        .iter()
        .find(|m| matches!(m.special, SpecialMove::DoublePawnStep))
        .copied()
        .expect("black pawn should have a double-step option from its start square");
    game.make_move(double_step);

    let white_moves = game.moves_for(Pos::new(2, 4));
    let ep = white_moves.iter().find(|m| matches!(m.special, SpecialMove::EnPassant { .. }));
    assert!(ep.is_some(), "white pawn should be able to capture en passant right after black's double step");
    let ep = ep.unwrap();
    assert_eq!(ep.to, Pos::new(3, 3));
    if let SpecialMove::EnPassant { captured_pawn } = ep.special {
        assert_eq!(captured_pawn, Pos::new(3, 4));
    }
}

// ---------- new: check / checkmate / stalemate / castle-safety / promotion ----------

#[test]
fn king_in_check_is_detected() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(3, 0), Piece { player: Player::Black, kind: PieceType::Rook });
    assert!(check::king_in_check(&board, Player::White, RaptorPachydermRules::FalconMammoth));
}

#[test]
fn legal_moves_excludes_a_pinned_pieces_moves_that_would_expose_the_king() {
    // White King and Bishop share a file with a Black Rook directly behind
    // the Bishop; the Bishop is pinned - every diagonal move takes it off
    // the file and exposes the King, so it should have zero legal moves.
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(3, 8), Piece { player: Player::White, kind: PieceType::Bishop });
    board.place_new(Pos::new(3, 0), Piece { player: Player::Black, kind: PieceType::Rook });

    let moves = check::legal_moves(&board, Player::White, RaptorPachydermRules::FalconMammoth);
    assert!(!moves.iter().any(|m| m.from == Pos::new(3, 8)), "the pinned bishop should have no legal moves");
}

fn ladder_mate_position() -> Board {
    // Two Black Rooks: one cuts off the entire rank in front of the King
    // (row 1), the other checks along the King's own back rank (row 0).
    // Every one of the King's 5 neighboring squares is covered by one rook
    // or the other, and neither rook is adjacent/capturable.
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 0), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 1), Piece { player: Player::Black, kind: PieceType::Rook });
    board.place_new(Pos::new(0, 0), Piece { player: Player::Black, kind: PieceType::Rook });
    board
}

#[test]
fn checkmate_is_detected() {
    let board = ladder_mate_position();
    assert_eq!(
        check::game_status(&board, Player::White, RaptorPachydermRules::FalconMammoth),
        check::GameStatus::Checkmate(Player::White),
    );

    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    match game.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose (checkmate) for the side to move, got {:?}", other),
    }
}

#[test]
#[should_panic]
fn actions_panics_once_the_game_is_already_over() {
    let game = Game::from_setup(ladder_mate_position(), RaptorPachydermRules::FalconMammoth);
    game.actions(&mut |_| {});
}

#[test]
fn stalemate_is_detected() {
    // King boxed into a corner with no legal moves, but not in check.
    let mut board = Board::empty();
    board.place_new(Pos::new(0, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(2, 8), Piece { player: Player::Black, kind: PieceType::Queen });

    assert!(!check::king_in_check(&board, Player::White, RaptorPachydermRules::FalconMammoth));
    assert_eq!(
        check::game_status(&board, Player::White, RaptorPachydermRules::FalconMammoth),
        check::GameStatus::Stalemate(Player::White),
    );

    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    match game.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw (stalemate), got {:?}", other),
    }
}

#[test]
fn castling_is_unavailable_while_the_king_is_in_check() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 9), Piece { player: Player::White, kind: PieceType::Rook });
    board.place_new(Pos::new(3, 0), Piece { player: Player::Black, kind: PieceType::Rook });

    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 9));
    assert!(!moves.iter().any(|m| matches!(m.special, SpecialMove::Castle { .. })));
}

#[test]
fn castling_is_unavailable_through_an_attacked_transit_square() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 9), Piece { player: Player::White, kind: PieceType::Rook });
    // Attacks (2,9), the square the king must cross to reach (1,9) - but
    // neither the king's own square (3,9) nor its landing square (1,9).
    board.place_new(Pos::new(2, 0), Piece { player: Player::Black, kind: PieceType::Rook });

    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 9));
    assert!(!moves.iter().any(|m| matches!(m.special, SpecialMove::Castle { .. })));
}

#[test]
fn pawn_promotes_to_queen_on_reaching_the_far_rank() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 1), Piece { player: Player::White, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let moves = game.moves_for(Pos::new(3, 1));
    let advance = moves.iter().find(|m| m.to == Pos::new(3, 0)).expect("pawn should be able to advance to the far rank");
    assert_eq!(advance.piece.kind, PieceType::Queen, "a pawn reaching the far rank should auto-promote");

    // Same rule via a diagonal capture-promotion.
    let mut board = Board::empty();
    board.place_new(Pos::new(2, 1), Piece { player: Player::White, kind: PieceType::Pawn });
    board.place_new(Pos::new(3, 0), Piece { player: Player::Black, kind: PieceType::Rook });
    let game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);
    let capture = game.moves_for(Pos::new(2, 1)).into_iter().find(|m| m.to == Pos::new(3, 0)).expect("pawn should be able to capture onto the far rank");
    assert_eq!(capture.piece.kind, PieceType::Queen);
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Game::new();
    let mut mcts = MCTS::new(g).with_custom_evaluation();
    mcts.ponder(200);
    assert!(mcts.best().is_some());
}
