use super::checkers::*;
use arbor::*;

fn empty_board() -> [Square;N] {
    [Square::Empty;N]
}

fn sorted(mut moves: Vec<Move>) -> Vec<Move> {
    moves.sort_by_key(|m| (m.from,m.to));
    moves
}

#[test]
fn initial_setup_piece_counts() {
    let g = Checkers::new();

    for sq in 0..12 {
        assert_eq!(g.square[sq],Square::Occupied(Side::Red,Kind::Man));
    }
    for sq in 12..20 {
        assert_eq!(g.square[sq],Square::Empty);
    }
    for sq in 20..32 {
        assert_eq!(g.square[sq],Square::Occupied(Side::White,Kind::Man));
    }

    assert_eq!(g.side,Side::Red);
}

#[test]
fn simple_forward_step_is_legal() {
    let mut board = empty_board();
    board[14] = Square::Occupied(Side::Red,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(sorted(moves),vec![
        Move {from: 14, to: 17},
        Move {from: 14, to: 18},
    ]);
}

#[test]
fn man_cannot_capture_backward() {
    let mut board = empty_board();
    board[14] = Square::Occupied(Side::Red,Kind::Man);
    // An enemy piece sits diagonally behind the man, with an empty landing
    // square beyond it. If men could capture backward this would be a
    // mandatory jump; American checkers men may only move/capture forward.
    board[9] = Square::Occupied(Side::White,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(sorted(moves),vec![
        Move {from: 14, to: 17},
        Move {from: 14, to: 18},
    ]);
}

#[test]
fn capture_is_mandatory() {
    let mut board = empty_board();
    board[14] = Square::Occupied(Side::Red,Kind::Man);
    board[18] = Square::Occupied(Side::White,Kind::Man);
    // landing square 23 beyond the capture is left empty

    // A second red man with only a quiet step available; the mandatory
    // capture rule should suppress it entirely.
    board[0] = Square::Occupied(Side::Red,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(moves,vec![Move {from: 14, to: 23}]);
}

#[test]
fn make_capture_removes_jumped_piece() {
    let mut board = empty_board();
    board[14] = Square::Occupied(Side::Red,Kind::Man);
    board[18] = Square::Occupied(Side::White,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);
    let next = g.make(Move {from: 14, to: 23});

    assert_eq!(next.square[14],Square::Empty);
    assert_eq!(next.square[18],Square::Empty);
    assert_eq!(next.square[23],Square::Occupied(Side::Red,Kind::Man));
}

fn multi_jump_board() -> Checkers {
    let mut board = empty_board();
    board[8] = Square::Occupied(Side::Red,Kind::Man);
    board[13] = Square::Occupied(Side::White,Kind::Man);
    // 17 (first landing) left empty
    board[22] = Square::Occupied(Side::White,Kind::Man);
    // 26 (second landing) left empty

    Checkers::debug_state(board,Side::Red)
}

#[test]
fn multi_jump_forces_continuation() {
    let g = multi_jump_board();

    let mut first = Vec::new();
    g.actions(&mut |m| first.push(m));
    assert_eq!(first,vec![Move {from: 8, to: 17}]);

    let g2 = g.make(Move {from: 8, to: 17});

    // The chain isn't finished: the same side must continue jumping with
    // the same piece.
    assert_eq!(g2.side,Side::Red);
    assert_eq!(g2.jumping,Some(17));

    let mut second = Vec::new();
    g2.actions(&mut |m| second.push(m));
    assert_eq!(second,vec![Move {from: 17, to: 26}]);
}

#[test]
fn multi_jump_completes_and_switches_side() {
    let g = multi_jump_board();
    let g2 = g.make(Move {from: 8, to: 17});
    let g3 = g2.make(Move {from: 17, to: 26});

    assert_eq!(g3.side,Side::White);
    assert_eq!(g3.jumping,None);
    assert_eq!(g3.square[13],Square::Empty);
    assert_eq!(g3.square[22],Square::Empty);
    assert_eq!(g3.square[26],Square::Occupied(Side::Red,Kind::Man));
}

#[test]
fn promotion_ends_turn_even_with_further_capture_available() {
    let mut board = empty_board();
    board[20] = Square::Occupied(Side::Red,Kind::Man);
    board[24] = Square::Occupied(Side::White,Kind::Man);
    // 29 (landing square, on Red's king row) left empty

    // Positioned so that, were the turn to continue, a further capture
    // would be available from square 29 - it must not be taken because
    // promotion ends the turn immediately.
    board[25] = Square::Occupied(Side::White,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));
    assert_eq!(moves,vec![Move {from: 20, to: 29}]);

    let g2 = g.make(Move {from: 20, to: 29});

    assert_eq!(g2.square[29],Square::Occupied(Side::Red,Kind::King));
    assert_eq!(g2.side,Side::White);
    assert_eq!(g2.jumping,None);
    // The piece that would have been jumped is untouched.
    assert_eq!(g2.square[25],Square::Occupied(Side::White,Kind::Man));
}

#[test]
fn king_can_capture_backward() {
    let mut board = empty_board();
    board[17] = Square::Occupied(Side::Red,Kind::King);
    board[13] = Square::Occupied(Side::White,Kind::Man);
    // 8 (landing square, behind the king) left empty

    let g = Checkers::debug_state(board,Side::Red);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));

    assert_eq!(moves,vec![Move {from: 17, to: 8}]);
}

#[test]
fn no_legal_moves_is_a_loss() {
    let mut board = empty_board();
    board[0] = Square::Occupied(Side::Red,Kind::Man);
    board[4] = Square::Occupied(Side::White,Kind::Man);
    board[5] = Square::Occupied(Side::White,Kind::Man);
    // 9 blocks the only capture landing square that would otherwise be
    // reachable by jumping the piece on square 5.
    board[9] = Square::Occupied(Side::White,Kind::Man);

    let g = Checkers::debug_state(board,Side::Red);

    match g.gameover() {
        Some(GameResult::Lose) => {},
        other => panic!("expected Lose for the blocked side to move, got {:?}",other),
    }
}

#[test]
fn no_progress_triggers_draw() {
    let mut board = empty_board();
    board[17] = Square::Occupied(Side::Red,Kind::King);
    board[13] = Square::Occupied(Side::White,Kind::Man);

    let mut g = Checkers::debug_state(board,Side::Red);
    g.no_progress = NO_PROGRESS_LIMIT;

    // Even though a capture is available, the no-progress counter forces a
    // draw so random MCTS rollouts are guaranteed to terminate.
    match g.gameover() {
        Some(GameResult::Draw) => {},
        other => panic!("expected Draw once the no-progress limit is reached, got {:?}",other),
    }
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Checkers::new();
    let mut mcts = MCTS::new(g);
    mcts.ponder(2000);
    assert!(mcts.best().is_some());
}
