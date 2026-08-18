use super::reversi::*;
use arbor::*;

//Local helpers mirroring the crate's private index scheme (row*8+col), since the BitBoard
//trait and its `space`/`iter` helpers are private to the reversi module.
fn idx(row: u32, col: u32) -> u64 {
    ((row << 3) | col) as u64
}

fn sq(row: u32, col: u32) -> u64 {
    1u64 << idx(row, col)
}

//Lowest `n` bits set (0..=64).
fn low_bits(n: u32) -> u64 {
    if n == 64 { u64::MAX } else { (1u64 << n) - 1 }
}

#[test]
fn new_game_starting_position() {
    let g = Reversi::new();
    assert_eq!(g.f.count_ones(), 2);
    assert_eq!(g.e.count_ones(), 2);
    assert_eq!(g.side, Disc::W);
    assert!(!g.pass);
}

#[test]
fn opening_position_offers_exactly_four_capture_moves() {
    let g = Reversi::new();
    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(moves.len(), 4);
    assert!(moves.iter().all(|m| matches!(m, Move::Capture(_))));
}

#[test]
fn single_direction_capture_flips_sandwiched_discs() {
    //Friend at (5,0), two enemies running north from the target at (2,0): target -> e -> e -> f.
    let target = sq(2, 0);
    let enemy_run = sq(3, 0) | sq(4, 0);
    let friend = sq(5, 0);

    let g = Reversi { f: friend, e: enemy_run, side: Disc::W, pass: false };
    let next = g.make(Move::Capture(idx(2, 0)));

    //Side flips, so the mover's discs (now the majority color on that column) live in `e`.
    assert_eq!(next.e, target | enemy_run | friend);
    assert_eq!(next.f, 0);
    assert_eq!(next.side, Disc::B);
}

#[test]
fn multi_direction_capture_flips_all_sandwiched_lines_at_once() {
    //Two independent sandwiches through the same target square: one running north, one running
    //east. Both should be captured by a single move.
    let target = sq(2, 2);
    let north_run = sq(3, 2) | sq(4, 2);
    let north_anchor = sq(5, 2);
    let east_run = sq(2, 3) | sq(2, 4);
    let east_anchor = sq(2, 5);

    let g = Reversi {
        f: north_anchor | east_anchor,
        e: north_run | east_run,
        side: Disc::W,
        pass: false,
    };
    let next = g.make(Move::Capture(idx(2, 2)));

    let expected_mover_discs = target | north_run | north_anchor | east_run | east_anchor;
    assert_eq!(next.e, expected_mover_discs);
    assert_eq!(next.f, 0);
}

#[test]
fn pass_is_forced_when_no_captures_are_available() {
    //Two isolated discs far apart: no adjacency, so no sandwich is ever possible.
    let g = Reversi { f: sq(0, 0), e: sq(7, 7), side: Disc::W, pass: false };

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(moves, vec![Move::Pass]);
}

#[test]
fn pass_move_swaps_sides_and_sets_the_pass_flag() {
    let g = Reversi { f: sq(0, 0), e: sq(7, 7), side: Disc::W, pass: false };
    let next = g.make(Move::Pass);

    assert_eq!(next.f, g.e);
    assert_eq!(next.e, g.f);
    assert_eq!(next.side, Disc::B);
    assert!(next.pass);
}

#[test]
fn double_pass_with_no_moves_available_ends_the_game() {
    //After the single isolated-discs position passes once, the resulting state itself has
    //pass == true and still no captures available for the new side to move - this is exactly
    //the "two consecutive passes" end condition.
    let g = Reversi { f: sq(0, 0), e: sq(7, 7), side: Disc::W, pass: false };
    let passed = g.make(Move::Pass);

    match passed.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw (one disc each), got {:?}", other),
    }
}

#[test]
fn full_board_win_for_the_side_with_more_discs() {
    let f = low_bits(33);
    let e = !f;
    let g = Reversi { f, e, side: Disc::W, pass: false };

    match g.gameover() {
        Some(GameResult::Win) => {}
        other => panic!("expected Win for the 33-disc majority side, got {:?}", other),
    }
}

#[test]
fn full_board_loss_for_the_side_with_fewer_discs() {
    let f = low_bits(20);
    let e = !f;
    let g = Reversi { f, e, side: Disc::W, pass: false };

    match g.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the 20-disc minority side, got {:?}", other),
    }
}

#[test]
fn full_board_draw_when_discs_are_split_evenly() {
    let f = low_bits(32);
    let e = !f;
    let g = Reversi { f, e, side: Disc::W, pass: false };

    match g.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw for a 32/32 split, got {:?}", other),
    }
}

#[test]
fn gameover_is_none_mid_game() {
    let g = Reversi::new();
    assert!(g.gameover().is_none());
}

#[test]
fn hash_is_stable_for_the_same_position() {
    let g = Reversi::new();
    assert_eq!(g.hash(), g.hash());
}

#[test]
fn hash_differs_after_a_real_move() {
    let g = Reversi::new();
    let action = {
        let mut first = None;
        g.actions(&mut |m| if first.is_none() { first = Some(m); });
        first.expect("opening position has legal moves")
    };
    let next = g.make(action);

    assert_ne!(g.hash(), next.hash());
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Reversi::new();
    let mut mcts = MCTS::new(g);
    mcts.ponder(2000);
    assert!(mcts.best().is_some());
}
