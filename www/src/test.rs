//Unit tests for the pure, non-DOM logic that drives the visual interface: the move-highlighting
//heatmap (util::colorize) and the two slider-label formatters (components::game_ui). None of
//this touches yew's Component/html! machinery or web-sys, so it runs as plain native `cargo
//test` - no wasm target or browser needed.
use crate::util::colorize;
use crate::components::game_ui::{fmt_ai_time,fmt_ai_eve,GameInstance};

#[test]
fn colorize_ranks_the_extremes_and_the_average_into_distinct_bands() {
    let weighted = vec![("max",1.0f32), ("mid",0.5), ("min",0.0)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    assert_eq!(classed,vec![("max","pos-75p"), ("mid","pos-0p"), ("min","neg-75p")]);
}

#[test]
fn colorize_preserves_the_order_and_identity_of_the_input() {
    let weighted = vec![(3,0.9f32), (1,0.1), (2,0.5)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    let ids: Vec<i32> = classed.iter().map(|(a,_)| *a).collect();
    assert_eq!(ids,vec![3,1,2]);
}

#[test]
fn colorize_clears_stale_entries_from_a_previous_call() {
    //classed is reused across renders (see GameUI::ponder) rather than freshly allocated each
    //time, so a caller that forgot to clear it first would otherwise see highlighting from the
    //previous board position bleed into the new one.
    let mut classed = vec![(999,"leftover-from-a-previous-position")];
    let weighted = vec![(1,0.5f32)];
    colorize(&weighted,&mut classed);

    assert_eq!(classed.len(),1);
    assert_ne!(classed[0].0,999);
}

#[test]
fn colorize_does_not_panic_on_an_empty_input() {
    let weighted: Vec<(i32,f32)> = Vec::new();
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);
    assert!(classed.is_empty());
}

//Characterization test, not an endorsement: when every action has exactly the same estimated
//value (a real, common case - e.g. right at the start of a search before any stats differ),
//`scale` divides out to 0.0 and every normalized value becomes 0.0/0.0 = NaN. Every `f < ...`
//comparison against NaN is false, so execution falls through to the final `else` and every
//single action gets painted "pos-75p" (strongest green) - visually claiming the AI has a clear
//favorite when in fact none of the options are distinguishable yet. This pins down the current
//behavior so a future change to the scaling logic is a deliberate choice, not a surprise.
#[test]
fn colorize_marks_every_action_as_strongly_favored_when_all_weights_are_tied() {
    let weighted = vec![(1,0.5f32), (2,0.5), (3,0.5)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    assert!(classed.iter().all(|(_,c)| *c == "pos-75p"),
        "expected the current (NaN-driven) all-green fallback, got {:?}",classed);
}

#[test]
fn fmt_ai_time_displays_the_raw_second_count() {
    assert_eq!(fmt_ai_time(1),"1");
    assert_eq!(fmt_ai_time(20),"20");
}

#[test]
fn fmt_ai_eve_scales_the_raw_slider_value_down_to_the_real_exploration_constant() {
    //The "Exploration" slider ranges 20..=40 (see GameUI::view's <Setting> for ai-eve);
    //fmt_ai_eve turns that raw tick count into the actual exploration constant that gets passed
    //to with_exploration (see GameUI::ponder, which divides by 20.0 the same way) - if these two
    //divisors ever drift apart, the displayed number would stop matching what the search is
    //actually using.
    assert_eq!(fmt_ai_eve(20),"1.00");
    assert_eq!(fmt_ai_eve(28),"1.40");
    assert_eq!(fmt_ai_eve(40),"2.00");
}

//Each game's GameInstance::status() names the win condition and the winning side in a game-
//appropriate way once gameover() fires. Each test below constructs a known terminal position
//(not driven through real play) and checks the exact rendered string - deliberately including
//the winner, not just "did it say something", since this exact family of code (find the current
//player, name "the other side" as the winner) is where a game's win/lose polarity is easiest to
//get backwards (found and fixed one real case in Connect4's own color labels while writing
//these: Disc::R/Disc::Y were mislabeled "White"/"Black" instead of "Red"/"Yellow").
//
//`use` statements are local to each test rather than at module scope because several of these
//games each define their own same-named types (Move, Side, Square, Disc, Column, ...) that
//would otherwise collide if imported together.

#[test]
fn tictactoe_status_names_the_winner_with_a_game_specific_message() {
    use tictactoe::{TicTacToe,Grid::*};
    // X completes the left column (TL,ML,BL) on the 5th move.
    let g = TicTacToe::load(&[TL,TM,ML,MM,BL]);
    assert_eq!(g.status(),"3 in a Row! - X Wins");
}

#[test]
fn tictactoe_status_names_a_draw() {
    use tictactoe::{TicTacToe,Grid::*};
    let g = TicTacToe::load(&[TL,TM,TR,MM,ML,MR,BM,BL,BR]);
    assert_eq!(g.status(),"Board Full! - Draw");
}

#[test]
fn reversi_status_names_the_winner_with_a_game_specific_message() {
    use reversi::{Reversi,Disc};
    let f = (1u64 << 33) - 1; // White holds 33 squares
    let e = !f;               // Black holds the remaining 31
    let g = Reversi { f, e, side: Disc::W, pass: false };
    assert_eq!(g.status(),"Most Discs! - White Wins");
}

#[test]
fn connect4_status_names_the_winner_with_a_game_specific_message() {
    use connect4::connect4::{Connect4,Column::*};
    // Disc::R (displayed as "Maroon") fills the bottom row at columns 1-4, completing a
    // horizontal 4-in-a-row.
    let g = Connect4::load(&[C1,C1,C2,C2,C3,C3,C4]);
    assert_eq!(g.status(),"4 in a Row! - Maroon Wins");
}

#[test]
fn onestone_status_names_a_corner_reached_win() {
    use onestone::{Onestone,Side,Square};
    let mut board = [Square::Empty; 25];
    board[24] = Square::Piece(Side::A,1); // A reached B's home corner, A's target
    board[0] = Square::Piece(Side::B,1);
    let g = Onestone::debug_state(board,Side::B,1);
    assert_eq!(g.status(),"Corner Reached! - Side-Blue Wins");
}

#[test]
fn onestone_status_names_an_all_captured_win() {
    use onestone::{Onestone,Side,Square};
    let mut board = [Square::Empty; 25];
    board[6] = Square::Piece(Side::A,1); // the only piece left on the board
    let g = Onestone::debug_state(board,Side::B,1);
    assert_eq!(g.status(),"All Captured! - Side-Blue Wins");
}

#[test]
fn chessbattle70_status_names_the_winner_with_a_game_specific_message() {
    use chessbattle70::{Game,Board,Pos,Piece,PieceType,Player,RaptorPachydermRules};
    // Ladder-mate: two Black Rooks cover every square around White's King.
    let mut board = Board::empty();
    board.place_new(Pos::new(3,0),Piece {player: Player::White, kind: PieceType::King});
    board.place_new(Pos::new(0,1),Piece {player: Player::Black, kind: PieceType::Rook});
    board.place_new(Pos::new(0,0),Piece {player: Player::Black, kind: PieceType::Rook});
    let g = Game::from_setup(board,RaptorPachydermRules::FalconMammoth);
    assert_eq!(g.status(),"Checkmate! - Black Wins");
}

#[test]
fn chessbattle70_status_names_a_stalemate_draw() {
    use chessbattle70::{Game,Board,Pos,Piece,PieceType,Player,RaptorPachydermRules};
    let mut board = Board::empty();
    board.place_new(Pos::new(0,9),Piece {player: Player::White, kind: PieceType::King});
    board.place_new(Pos::new(2,8),Piece {player: Player::Black, kind: PieceType::Queen});
    let g = Game::from_setup(board,RaptorPachydermRules::FalconMammoth);
    assert_eq!(g.status(),"Stalemate! - Draw");
}
