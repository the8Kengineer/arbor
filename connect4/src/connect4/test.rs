use super::*;

fn idx(row: usize, col: usize) -> usize { row*W + col }

#[test]
fn new_game_starting_position() {
    let g = Connect4::new();
    assert!(g.space.iter().all(|d| *d == Disc::N));
    assert_eq!(g.player(), Disc::R);
    assert!(g.gameover().is_none());
}

#[test]
fn actions_offers_all_seven_columns_on_an_empty_board() {
    let g = Connect4::new();
    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));
    assert_eq!(moves, COL.to_vec());
}

#[test]
fn gravity_stacks_discs_from_the_bottom_of_a_column() {
    let g = Connect4::load(&[C1,C1]);
    assert_eq!(g.space[idx(0,0)], Disc::R);
    assert_eq!(g.space[idx(1,0)], Disc::Y);
}

#[test]
fn actions_excludes_a_full_column_but_offers_the_rest() {
    //Alternating drops into the same column never form a vertical 4-in-a-row (R,Y,R,Y,R,Y), so
    //this fills the column without ending the game.
    let g = Connect4::load(&[C1,C1,C1,C1,C1,C1]);
    assert!(g.gameover().is_none());

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));
    assert!(!moves.contains(&C1));
    assert_eq!(moves.len(), 6);
}

#[test]
fn vertical_four_in_a_row_is_detected() {
    let mut space = [Disc::N; W*H];
    space[idx(0,0)] = Disc::R;
    space[idx(1,0)] = Disc::R;
    space[idx(2,0)] = Disc::R;
    let g = Connect4 { space, gameover: false, side: true, winner: Disc::N, hash: 0 };

    let next = g.make(C1);
    match next.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the side to move after the vertical win, got {:?}", other),
    }
}

#[test]
fn horizontal_four_in_a_row_is_detected() {
    let mut space = [Disc::N; W*H];
    space[idx(0,0)] = Disc::R;
    space[idx(0,1)] = Disc::R;
    space[idx(0,2)] = Disc::R;
    let g = Connect4 { space, gameover: false, side: true, winner: Disc::N, hash: 0 };

    let next = g.make(C4);
    match next.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the side to move after the horizontal win, got {:?}", other),
    }
}

#[test]
fn diagonal_ascending_four_in_a_row_is_detected() {
    //Builds the "/" diagonal (0,0)-(1,1)-(2,2)-(3,3) for Red, with just enough filler beneath
    //each cell for gravity to let the piece actually land there.
    let mut space = [Disc::N; W*H];
    space[idx(0,0)] = Disc::R;
    space[idx(0,1)] = Disc::Y; space[idx(1,1)] = Disc::R;
    space[idx(0,2)] = Disc::Y; space[idx(1,2)] = Disc::Y; space[idx(2,2)] = Disc::R;
    space[idx(0,3)] = Disc::Y; space[idx(1,3)] = Disc::Y; space[idx(2,3)] = Disc::Y;
    let g = Connect4 { space, gameover: false, side: true, winner: Disc::N, hash: 0 };

    let next = g.make(C4);
    match next.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the side to move after the ascending diagonal win, got {:?}", other),
    }
}

#[test]
fn diagonal_descending_four_in_a_row_is_detected() {
    //Builds the "\" diagonal (0,3)-(1,2)-(2,1)-(3,0) for Red, same filler approach.
    let mut space = [Disc::N; W*H];
    space[idx(0,3)] = Disc::R;
    space[idx(0,2)] = Disc::Y; space[idx(1,2)] = Disc::R;
    space[idx(0,1)] = Disc::Y; space[idx(1,1)] = Disc::Y; space[idx(2,1)] = Disc::R;
    space[idx(0,0)] = Disc::Y; space[idx(1,0)] = Disc::Y; space[idx(2,0)] = Disc::Y;
    let g = Connect4 { space, gameover: false, side: true, winner: Disc::N, hash: 0 };

    let next = g.make(C1);
    match next.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected Lose for the side to move after the descending diagonal win, got {:?}", other),
    }
}

fn creates_run_of_four(space: &[Disc; W*H], r: usize, c: usize, color: Disc) -> bool {
    let get = |r: i32, c: i32| -> Disc {
        if r < 0 || c < 0 || r as usize >= H || c as usize >= W {
            Disc::N
        } else {
            space[r as usize * W + c as usize]
        }
    };

    for (dr,dc) in [(0i32,1i32),(1,0),(1,1),(1,-1)] {
        let mut run = 1;
        let (mut rr,mut cc) = (r as i32 - dr, c as i32 - dc);
        while get(rr,cc) == color { run += 1; rr -= dr; cc -= dc; }
        let (mut rr2,mut cc2) = (r as i32 + dr, c as i32 + dc);
        while get(rr2,cc2) == color { run += 1; rr2 += dr; cc2 += dc; }
        if run >= 4 { return true; }
    }
    false
}

//Backtracking search over every cell except (H-1,W-1) in row-major order, at each cell trying
//a color that doesn't complete a run of 4 in any direction and backtracking on a dead end. Row-
//major order guarantees every earlier cell in any potential run has already been decided, so
//any board this returns can never contain a run of 4. Leaves the very last cell for the test
//itself to fill via a real make() call, so the test exercises the actual "board is now full"
//detection in make() rather than asserting it by fiat.
fn fill(space: &mut [Disc; W*H], cells: &[(usize,usize)]) -> bool {
    let Some((&(r,c), rest)) = cells.split_first() else { return true; };
    for color in [Disc::R, Disc::Y] {
        if !creates_run_of_four(space,r,c,color) {
            space[idx(r,c)] = color;
            if fill(space,rest) { return true; }
            space[idx(r,c)] = Disc::N;
        }
    }
    false
}

fn full_board_minus_last_cell() -> [Disc; W*H] {
    let cells: Vec<(usize,usize)> = (0..H).flat_map(|r| (0..W).map(move |c| (r,c)))
        .filter(|&(r,c)| !(r == H-1 && c == W-1))
        .collect();

    let mut space = [Disc::N; W*H];
    assert!(fill(&mut space,&cells),"could not find any run-4-free arrangement for the board minus one cell");
    space
}

#[test]
fn full_board_with_no_winner_is_a_draw() {
    let space = full_board_minus_last_cell();
    let side = !creates_run_of_four(&space,H-1,W-1,Disc::R);
    let g = Connect4 { space, gameover: false, side, winner: Disc::N, hash: 0 };

    let next = g.make(C7);
    match next.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw on a full board with no winner, got {:?}",other),
    }
}

#[test]
fn hash_is_stable_for_the_same_position() {
    let g = Connect4::new();
    assert_eq!(g.hash(), g.hash());
}

#[test]
fn hash_differs_after_a_real_move() {
    let g = Connect4::new();
    let next = g.make(C1);
    assert_ne!(g.hash(), next.hash());
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Connect4::new();
    let mut mcts = MCTS::new(g);
    mcts.ponder(2000);
    assert!(mcts.best().is_some());
}
