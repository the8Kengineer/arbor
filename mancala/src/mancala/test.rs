use super::*;

fn best(moves: &[Pit]) -> Pit {

    let game = Mancala::load(&moves);
    let mut mcts = MCTS::new(game).with_transposition();
    mcts.ponder(10000);
    mcts.best().expect("Should find a best action")
}

#[test]
fn mancala_free_move_1() {
    let mut game = Mancala::new();
    println!("{}",game);

    game = game.make(Pit::R3);
    println!("{}",game);

    game = game.make(Pit::R6);
    println!("{}",game);

    assert!(game.side == super::Player::L);
    assert!(game.pit[RB] == 2);
    assert!(game.pit[LB] == 0);
}

#[test]
fn mancala_free_move_2() {
    let mut game = Mancala::new();
    println!("{}",game);

    game = game.make(Pit::R3);
    println!("{}",game);

    game = game.make(Pit::R6);
    println!("{}",game);

    game = game.make(Pit::L2);
    println!("{}",game);

    game = game.make(Pit::L6);
    println!("{}",game);


    assert!(game.side == super::Player::R);
    assert!(game.pit[RB] == 2);
    assert!(game.pit[LB] == 2);
}

#[test]
fn mancala_right_capture() {
    let mut game = Mancala::new();
    println!("{}",game);

    game = game.make(Pit::R6);
    println!("{}",game);

    game = game.make(Pit::L6);
    println!("{}",game);

    game = game.make(Pit::R1);
    println!("{}",game);

    assert!(game.side == super::Player::L);
    assert!(game.pit[RB] == 1 + 1 + 5);
    assert!(game.pit[LB] == 1);
    assert!(game.pit[Pit::R6 as usize] == 0);
    assert!(game.pit[Pit::L1 as usize] == 0);
}

#[test]
fn mancala_left_capture() {
    let mut game = Mancala::new();
    println!("{}",game);

    game = game.make(Pit::R6);
    println!("{}",game);

    game = game.make(Pit::L2);
    println!("{}",game);

    game = game.make(Pit::L6);
    println!("{}",game);

    game = game.make(Pit::R5);
    println!("{}",game);

    game = game.make(Pit::L5);
    println!("{}",game);

    game = game.make(Pit::R2);
    println!("{}",game);

    game = game.make(Pit::L3);
    println!("{}",game);

    game = game.make(Pit::R1);
    println!("{}",game);

    game = game.make(Pit::L2);
    println!("{}",game);

    assert!(game.side == super::Player::R);
    assert!(game.pit[RB] == 4);
    assert!(game.pit[LB] == 12);
    assert!(game.pit[Pit::R4 as usize] == 0);
    assert!(game.pit[Pit::L3 as usize] == 0);
}

#[test]
fn mancala_best_move_split() {
    let m = best(&[R6,L6]);
    assert!((m == R1) || (m == R2));
}

#[test]
fn mancala_best_move_free_turn() {
    let m = best(&[R6,L6,R2]);
    assert!(m == R6);
}

//A move that both empties the mover's last remaining pit via a capture AND leaves the mover's
//whole side empty triggers the starvation/sweep rule: the opponent's leftover stones (which can
//no longer be legally played by anyone, since it's the mover's turn again next and they have
//nothing left) are swept into the opponent's own bank and every remaining pit clears to zero.
#[test]
fn starvation_sweep_awards_the_untouched_sides_remaining_stones_to_its_own_bank() {
    let mut pit = [0u8; NP];
    pit[Pit::L4 as usize] = 1; //the only stone left on the left side
    pit[Pit::R1 as usize] = 2;
    pit[Pit::R2 as usize] = 5; //lands opposite L4's landing square - gets captured
    pit[Pit::R3 as usize] = 3;

    let game = Mancala { pit, side: Player::L };
    let next = game.make(Pit::L4);

    for p in [Pit::L1,Pit::L2,Pit::L3,Pit::L4,Pit::L5,Pit::L6,Pit::R1,Pit::R2,Pit::R3,Pit::R4,Pit::R5,Pit::R6] {
        assert_eq!(next.pit[p as usize], 0, "{:?} should have been swept to 0", p);
    }
    assert_eq!(next.pit[LB], 6, "the capture bonus (5 captured + 1 landed) should credit the left bank");
    assert_eq!(next.pit[RB], 5, "the swept right-side remainder should credit the right bank");
}

#[test]
fn gameover_is_a_draw_when_both_banks_hold_the_same_total() {
    let mut pit = [0u8; NP];
    pit[LB] = 24;
    pit[RB] = 24;
    let game = Mancala { pit, side: Player::L };

    match game.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw when both banks are equal, got {:?}", other),
    }
}

#[test]
fn gameover_is_a_win_when_the_side_to_move_has_the_larger_bank() {
    let mut pit = [0u8; NP];
    pit[LB] = 30;
    pit[RB] = 18;
    let game = Mancala { pit, side: Player::L };

    match game.gameover() {
        Some(GameResult::Win) => {}
        other => panic!("expected Win for the side to move with the larger bank, got {:?}", other),
    }
}

#[test]
fn gameover_is_a_loss_when_the_side_to_move_has_the_smaller_bank() {
    let mut pit = [0u8; NP];
    pit[LB] = 10;
    pit[RB] = 38;
    let game = Mancala { pit, side: Player::L };

    match game.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the side to move with the smaller bank, got {:?}", other),
    }
}

#[test]
fn actions_never_offers_a_pit_that_currently_has_zero_stones() {
    let mut pit = [0u8; NP];
    pit[Pit::R1 as usize] = 0;
    pit[Pit::R2 as usize] = 3;
    pit[Pit::R3 as usize] = 0;
    pit[Pit::R4 as usize] = 1;
    pit[Pit::R5 as usize] = 0;
    pit[Pit::R6 as usize] = 2;
    let game = Mancala { pit, side: Player::R };

    let mut offered = Vec::new();
    game.actions(&mut |p| offered.push(p));
    offered.sort_by_key(|p| *p as usize);

    assert_eq!(offered, vec![Pit::R2,Pit::R4,Pit::R6]);
}

#[test]
fn hash_is_stable_for_the_same_position() {
    let game = Mancala::new();
    assert_eq!(game.hash(), game.hash());
}

#[test]
fn hash_differs_after_a_real_move() {
    let game = Mancala::new();
    let next = game.make(Pit::R3);
    assert_ne!(game.hash(), next.hash());
}
