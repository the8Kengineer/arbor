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

// ---------- random opening army (Game::new's actual starting position) ----------

#[test]
fn random_setup_obeys_the_design_docs_rank_constraints() {
    let king_col = WIDTH / 2;

    for seed in [1u64, 2, 42, 9999, u64::MAX] {
        let board = setup::random_setup(seed);
        for &(player, back_row, middle_row, front_row) in &[
            (Player::Black, 0, 1, 2),
            (Player::White, HEIGHT - 1, HEIGHT - 2, HEIGHT - 3),
        ] {
            for col in 0..WIDTH {
                let back = board.get(Pos::new(col, back_row)).unwrap().kind;
                let middle = board.get(Pos::new(col, middle_row)).unwrap().kind;
                let front = board.get(Pos::new(col, front_row)).unwrap().kind;

                if col == king_col {
                    assert_eq!(back, PieceType::King, "seed {}: {:?}'s king should be dead-centre of the back rank", seed, player);
                } else {
                    assert_ne!(back, PieceType::King, "seed {}: only one king per side", seed);
                    assert_ne!(back, PieceType::Pawn, "seed {}: back rank cannot have pawns", seed);
                }

                assert_ne!(middle, PieceType::King, "seed {}: middle rank cannot have a king", seed);

                assert!(
                    matches!(front, PieceType::Pawn | PieceType::Knight | PieceType::Bishop | PieceType::Falcon),
                    "seed {}: front rank piece {:?} is outside the allowed set", seed, front
                );
            }
        }
    }
}

#[test]
fn random_setup_varies_with_the_seed() {
    let a = setup::random_setup(1);
    let b = setup::random_setup(2);
    assert_ne!(a, b, "different seeds should produce different armies");
}

#[test]
fn random_setup_gives_each_side_an_independently_different_mix() {
    let board = setup::random_setup(1);

    let ranks_for = |rows: [i8; 3]| -> Vec<PieceType> {
        (0..WIDTH)
            .flat_map(|col| rows.into_iter().map(move |row| board.get(Pos::new(col, *row)).unwrap().kind))
            .collect()
    };

    let black = ranks_for([0, 1, 2]);
    let white = ranks_for([HEIGHT - 1, HEIGHT - 2, HEIGHT - 3]);
    assert_ne!(black, white, "the two sides should not end up with the same piece mix");
}

#[test]
fn new_game_always_starts_in_play_regardless_of_the_random_setup() {
    // Game::new draws a fresh random army each call; run it several times rather than trusting
    // a single roll of the dice.
    for _ in 0..20 {
        let g = Game::new();
        assert!(g.gameover().is_none(), "a random opening position should never already be over");

        let mut count = 0;
        g.actions(&mut |_| count += 1);
        assert!(count > 0, "White should always have at least one legal opening move");
    }
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Game::new();
    let mut mcts = MCTS::new(g).with_custom_evaluation();
    mcts.ponder(200);
    assert!(mcts.best().is_some());
}

// ---------- SeirawanHarper and Realism rule variants ----------
//
// Every test above plays under FalconMammoth (arbor's default and the only
// ruleset `Game::new()` wires up); the other two documented rulesets -
// SeirawanHarper (Hawk = Knight+Bishop, Elephant = Knight+Rook, both
// normally blocked) and Realism (a "flying" diagonal Hawk, and an Elephant
// whose knight-shaped move needs a clear L-path) - had no coverage at all.

#[test]
fn seirawan_harper_hawk_combines_knight_leaps_and_bishop_slides() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 5), Piece { player: Player::White, kind: PieceType::Falcon });
    // Blocks the bishop-slide component along one diagonal; the knight-leap
    // component must be unaffected (leaps ignore blockers), and the other
    // three diagonal directions must still slide freely.
    board.place_new(Pos::new(4, 6), Piece { player: Player::White, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::SeirawanHarper);
    let destinations: Vec<Pos> = game.moves_for(Pos::new(3, 5)).iter().map(|m| m.to).collect();

    for d in [
        Pos::new(4, 7), Pos::new(5, 6), Pos::new(5, 4), Pos::new(4, 3),
        Pos::new(2, 3), Pos::new(1, 4), Pos::new(1, 6), Pos::new(2, 7),
    ] {
        assert!(destinations.contains(&d), "expected a knight-shaped leap to {:?}", d);
    }

    // Bishop-shaped slide toward the friendly blocker stops before it.
    assert!(!destinations.contains(&Pos::new(4, 6)));
    assert!(!destinations.contains(&Pos::new(5, 7)));

    // An unblocked diagonal direction still slides all the way to the edge.
    assert!(destinations.contains(&Pos::new(4, 4)));
    assert!(destinations.contains(&Pos::new(5, 3)));
    assert!(destinations.contains(&Pos::new(6, 2)));
}

#[test]
fn seirawan_harper_elephant_combines_knight_leaps_and_rook_slides() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 5), Piece { player: Player::White, kind: PieceType::Mammoth });
    // Blocks the rook-slide component in one direction only.
    board.place_new(Pos::new(4, 5), Piece { player: Player::White, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::SeirawanHarper);
    let destinations: Vec<Pos> = game.moves_for(Pos::new(3, 5)).iter().map(|m| m.to).collect();

    for d in [
        Pos::new(4, 7), Pos::new(5, 6), Pos::new(5, 4), Pos::new(4, 3),
        Pos::new(2, 3), Pos::new(1, 4), Pos::new(1, 6), Pos::new(2, 7),
    ] {
        assert!(destinations.contains(&d), "expected a knight-shaped leap to {:?}", d);
    }

    assert!(!destinations.contains(&Pos::new(4, 5)), "blocked by a friendly piece");
    assert!(!destinations.contains(&Pos::new(5, 5)), "can't slide past the blocker");
    assert!(destinations.contains(&Pos::new(2, 5)), "unblocked rook direction");
    assert!(destinations.contains(&Pos::new(0, 5)), "unblocked rook direction slides to the edge");
}

#[test]
fn realism_hawk_flies_over_blockers_on_its_diagonal_component() {
    let mut board = Board::empty();
    board.place_new(Pos::new(0, 0), Piece { player: Player::White, kind: PieceType::Falcon });
    board.place_new(Pos::new(2, 2), Piece { player: Player::Black, kind: PieceType::Pawn });
    board.place_new(Pos::new(4, 4), Piece { player: Player::White, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::Realism);
    let moves = game.moves_for(Pos::new(0, 0));
    let destinations: Vec<Pos> = moves.iter().map(|m| m.to).collect();

    // The knight-shaped component still leaps normally.
    assert!(destinations.contains(&Pos::new(1, 2)));
    assert!(destinations.contains(&Pos::new(2, 1)));

    // The diagonal component flies clean over the enemy pawn, capturing it
    // as one of several possible destinations along the ray rather than
    // stopping there...
    assert!(destinations.contains(&Pos::new(1, 1)));
    let capture = moves.iter().find(|m| m.to == Pos::new(2, 2)).expect("should be able to land on/capture the enemy pawn");
    assert_eq!(capture.captured.unwrap().0, Pos::new(2, 2));
    assert!(destinations.contains(&Pos::new(3, 3)));

    // ...and also flies straight over the friendly pawn (never landing on
    // its square) to reach squares beyond it.
    assert!(!destinations.contains(&Pos::new(4, 4)), "can't land on a friendly piece");
    assert!(destinations.contains(&Pos::new(5, 5)), "flies over the friendly pawn");
    assert!(destinations.contains(&Pos::new(6, 6)));
}

#[test]
fn realism_elephant_requires_at_least_one_clear_knight_path() {
    // Two ways to trace the L from (1,1) to (3,2): via (2,1)+(3,1), or via
    // (1,2)+(2,2). Blocking one square from each route blocks the move.
    let mut board = Board::empty();
    board.place_new(Pos::new(1, 1), Piece { player: Player::White, kind: PieceType::Mammoth });
    board.place_new(Pos::new(2, 1), Piece { player: Player::Black, kind: PieceType::Pawn });
    board.place_new(Pos::new(1, 2), Piece { player: Player::Black, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::Realism);
    assert!(
        !game.moves_for(Pos::new(1, 1)).iter().any(|m| m.to == Pos::new(3, 2)),
        "both L-routes are blocked"
    );

    // Clearing just the (1,2) route leaves the other one fully open.
    let mut board = Board::empty();
    board.place_new(Pos::new(1, 1), Piece { player: Player::White, kind: PieceType::Mammoth });
    board.place_new(Pos::new(2, 1), Piece { player: Player::Black, kind: PieceType::Pawn });
    let game = Game::from_setup(board, RaptorPachydermRules::Realism);
    assert!(
        game.moves_for(Pos::new(1, 1)).iter().any(|m| m.to == Pos::new(3, 2)),
        "the route via (1,2)->(2,2) is clear"
    );
}

// ---------- castling application, king-adjacency check, and edge guards ----------

#[test]
fn castling_actually_relocates_the_rook_and_passes_the_turn() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 9), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 9), Piece { player: Player::White, kind: PieceType::Rook });
    let mut game = Game::from_setup(board, RaptorPachydermRules::FalconMammoth);

    // Go through check::legal_moves (not Game::moves_for, used everywhere
    // else in this file) so the castling move is actually simulated by
    // check::apply - moves_for alone never exercises that code path.
    let legal = check::legal_moves(&game.board, game.current_player, game.rules);
    let castle = legal.iter()
        .find(|m| matches!(m.special, SpecialMove::Castle {..}))
        .copied()
        .expect("castling should be legal here");

    game.make_move(castle);

    assert_eq!(game.board.get(Pos::new(3, 9)), None, "king's start square should be empty");
    assert_eq!(game.board.get(Pos::new(1, 9)).map(|p| p.kind), Some(PieceType::King));
    assert_eq!(game.board.get(Pos::new(0, 9)), None, "rook's start square should be empty");
    assert_eq!(game.board.get(Pos::new(2, 9)).map(|p| p.kind), Some(PieceType::Rook));
    assert_eq!(game.current_player, Player::Black);
}

#[test]
fn king_in_check_from_an_adjacent_enemy_king() {
    let mut board = Board::empty();
    board.place_new(Pos::new(3, 5), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(3, 6), Piece { player: Player::Black, kind: PieceType::King });
    assert!(check::king_in_check(&board, Player::White, RaptorPachydermRules::FalconMammoth));

    let mut board = Board::empty();
    board.place_new(Pos::new(3, 5), Piece { player: Player::White, kind: PieceType::King });
    board.place_new(Pos::new(0, 0), Piece { player: Player::Black, kind: PieceType::King });
    assert!(!check::king_in_check(&board, Player::White, RaptorPachydermRules::FalconMammoth));
}

#[test]
fn king_in_check_is_false_when_the_king_is_missing() {
    // Shouldn't happen in a real game (checkmate ends things first), but
    // king_in_check documents that it returns false rather than panicking.
    let board = Board::empty();
    assert!(!check::king_in_check(&board, Player::White, RaptorPachydermRules::FalconMammoth));
}

#[test]
fn board_get_and_meta_return_defaults_for_an_out_of_bounds_position() {
    let board = Board::empty();
    for p in [Pos::new(-1, 0), Pos::new(WIDTH, 0), Pos::new(0, -1), Pos::new(0, HEIGHT)] {
        assert_eq!(board.get(p), None);
        assert_eq!(board.meta(p), crate::board::SquareMeta::default());
    }
}

#[test]
fn piece_type_code_and_number_match_the_design_docs_table() {
    let table: [(PieceType, char, u8); 8] = [
        (PieceType::Pawn, 'P', 1),
        (PieceType::Knight, 'N', 2),
        (PieceType::Bishop, 'B', 3),
        (PieceType::Falcon, 'F', 4),
        (PieceType::Mammoth, 'M', 5),
        (PieceType::Rook, 'R', 6),
        (PieceType::Queen, 'Q', 7),
        (PieceType::King, 'K', 8),
    ];
    for (kind, code, number) in table {
        assert_eq!(kind.code(), code, "{:?}'s single-letter code", kind);
        assert_eq!(kind.number(), number, "{:?}'s design-doc 1-8 number", kind);
    }
}

#[test]
fn default_game_is_a_fresh_in_progress_random_game() {
    let g = Game::default();
    assert!(g.gameover().is_none());
    assert_eq!(g.current_player, Player::White);
}
