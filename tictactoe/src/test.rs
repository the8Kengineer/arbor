use super::tictactoe::Grid::*;
use super::tictactoe::*;
use arbor::*;

fn best(moves: &[Grid]) -> Grid {
    let game = TicTacToe::load(&moves);
    let mut mcts = MCTS::new(game).with_transposition();
    mcts.ponder(10000);
    mcts.best().expect("Should find a best action")
}

#[test]
fn tictactoe_best_obvious() {
    assert!(best(&[MM,TM,MR,ML,BR,TR]) == TL);
}

#[test]
fn tictactoe_best_even() {
    assert!(best(&[TL,MM,ML]) == BL);
}

#[test]
fn tictactoe_best_even2() {
    assert!(best(&[MM,ML,MR,TL]) == BL);
}

#[test]
fn tictactoe_best_split() {
    let m = best(&[MM,TM,MR,ML]);
    assert!((m == BR) || (m == TR));
}

const LINES: [[Grid;3];8] = [
    [TL,TM,TR],
    [ML,MM,MR],
    [BL,BM,BR],
    [TL,ML,BL],
    [TM,MM,BM],
    [TR,MR,BR],
    [TL,MM,BR],
    [TR,MM,BL],
];

//X only ever holds 2 marks before the 5th (final) move of this construction, so it's
//impossible for an earlier move to complete a line by accident - a completed line needs 3.
#[test]
fn winner_is_detected_on_all_eight_lines() {
    for line in LINES.iter() {
        let others: Vec<Grid> = ALLMOVES.iter().copied().filter(|g| !line.contains(g)).take(2).collect();
        let moves = [line[0], others[0], line[1], others[1], line[2]];
        let g = TicTacToe::load(&moves);

        match g.gameover() {
            Some(GameResult::Lose) => {}
            other => panic!("line {:?} did not produce Lose for the side to move, got {:?}", line, other),
        }
    }
}

#[test]
fn the_side_about_to_move_is_the_one_that_just_lost() {
    //X completes the left column (TL,ML,BL) on the 5th move.
    let g = TicTacToe::load(&[TL,TM,ML,MM,BL]);

    assert_eq!(g.side, Mark::O);
    match g.gameover() {
        Some(GameResult::Lose) => {}
        other => panic!("expected O (about to move) to have Lose since X just won, got {:?}", other),
    }
}

#[test]
fn draw_after_nine_moves_with_no_winner() {
    let g = TicTacToe::load(&[TL,TM,TR,MM,ML,MR,BM,BL,BR]);

    match g.gameover() {
        Some(GameResult::Draw) => {}
        other => panic!("expected a Draw, got {:?}", other),
    }
}

#[test]
fn gameover_is_none_mid_game() {
    let g = TicTacToe::load(&[TL,TM,MM]);
    assert!(g.gameover().is_none());
}

#[test]
fn actions_only_returns_empty_cells() {
    let g = TicTacToe::load(&[TL,TM,MM]);
    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert!(!moves.contains(&TL));
    assert!(!moves.contains(&TM));
    assert!(!moves.contains(&MM));
    assert_eq!(moves.len(), 6);
}

//hash() OR-accumulates a per-cell bit regardless of move order, so two different orderings
//that land on the same final X/O assignment are indistinguishable - and PartialEq is defined
//purely in terms of that hash.
#[test]
fn hash_is_order_independent_for_the_same_final_board() {
    let order_a = [TL,TM,ML,MM,TR];
    let order_b = [TL,MM,ML,TM,TR];
    let a = TicTacToe::load(&order_a);
    let b = TicTacToe::load(&order_b);

    assert_eq!(a.hash(), b.hash());
    assert!(a == b);
}

#[test]
#[should_panic(expected = "Make called on invalid space")]
fn make_panics_when_placing_on_an_occupied_cell() {
    let g = TicTacToe::load(&[TL]);
    g.make(TL);
}

#[test]
#[should_panic(expected = "Make called while gameover")]
fn make_panics_after_the_game_is_already_over() {
    let g = TicTacToe::load(&[TL,TM,ML,MM,BL]); // X already won the left column
    g.make(BR);
}