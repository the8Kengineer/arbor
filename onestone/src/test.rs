use super::onestone::*;
use arbor::*;

#[test]
fn initial_setup_side_a() {
    let g = Onestone::with_seed(42);
    assert_eq!(g.board[0], Square::Piece(Side::A,1));
    assert_eq!(g.board[1], Square::Piece(Side::A,2));
    assert_eq!(g.board[2], Square::Piece(Side::A,4));
    assert_eq!(g.board[5], Square::Piece(Side::A,3));
    assert_eq!(g.board[6], Square::Piece(Side::A,5));
    assert_eq!(g.board[10], Square::Piece(Side::A,6));
    assert_eq!(g.side, Side::A);
}

#[test]
fn initial_setup_side_b() {
    let g = Onestone::with_seed(42);
    assert_eq!(g.board[14], Square::Piece(Side::B,6));
    assert_eq!(g.board[18], Square::Piece(Side::B,5));
    assert_eq!(g.board[19], Square::Piece(Side::B,3));
    assert_eq!(g.board[22], Square::Piece(Side::B,4));
    assert_eq!(g.board[23], Square::Piece(Side::B,2));
    assert_eq!(g.board[24], Square::Piece(Side::B,1));
}

#[test]
fn not_gameover_in_normal_position() {
    let g = Onestone::with_seed(7);
    assert!(g.gameover().is_none());
}

#[test]
fn actions_respect_bounds_and_own_piece_blocking() {
    let mut board = [Square::Empty;25];
    board[6] = Square::Piece(Side::A,1);
    board[11] = Square::Piece(Side::A,2); // blocks the "down" move
    board[7] = Square::Piece(Side::B,3);  // capturable via the "right" move
    // index 12 (diagonal) stays empty, so that move is legal too

    let g = Onestone::debug_state(board,Side::A,1);

    let mut moves = Vec::new();
    g.actions(&mut |m| moves.push(m));
    moves.sort_by_key(|m| m.to);

    assert_eq!(moves,vec![
        Move {number: 1, to: 7},
        Move {number: 1, to: 12},
    ]);
}

#[test]
fn make_captures_opponent_piece() {
    let mut board = [Square::Empty;25];
    board[6] = Square::Piece(Side::A,1);
    board[7] = Square::Piece(Side::B,3);

    let g = Onestone::debug_state(board,Side::A,1);
    let next = g.make(Move {number: 1, to: 7});

    assert_eq!(next.board[7], Square::Piece(Side::A,1));
    assert_eq!(next.board[6], Square::Empty);

    let b_count = next.board.iter()
        .filter(|sq| matches!(sq, Square::Piece(Side::B,_)))
        .count();
    assert_eq!(b_count,0);
}

#[test]
fn die_fallback_to_nearest_neighbors() {
    let mut board = [Square::Empty;25];
    // Side A has cubes 2 and 5 remaining; a roll of 3 (missing) should offer both neighbors.
    board[0] = Square::Piece(Side::A,2);
    board[6] = Square::Piece(Side::A,5);

    let g = Onestone::debug_state(board,Side::A,3);

    let mut numbers_moved: Vec<u8> = Vec::new();
    g.actions(&mut |m| numbers_moved.push(m.number));
    numbers_moved.sort();
    numbers_moved.dedup();

    assert_eq!(numbers_moved,vec![2,5]);
}

#[test]
fn die_fallback_single_neighbor_when_only_one_side_available() {
    let mut board = [Square::Empty;25];
    // Only cube 2 remains; a roll of 6 has no higher neighbor, only a lower one.
    board[0] = Square::Piece(Side::A,2);

    let g = Onestone::debug_state(board,Side::A,6);

    let mut numbers_moved: Vec<u8> = Vec::new();
    g.actions(&mut |m| numbers_moved.push(m.number));
    numbers_moved.sort();
    numbers_moved.dedup();

    assert_eq!(numbers_moved,vec![2]);
}

#[test]
fn gameover_when_mover_reaches_target_corner() {
    let mut board = [Square::Empty;25];
    board[24] = Square::Piece(Side::A,1); // A reached B's home corner, A's target
    board[0] = Square::Piece(Side::B,1);

    // After A's move it becomes B's turn; gameover() is evaluated from the perspective of the
    // side about to move (B), mirroring tictactoe's "side to play last always wins" pattern.
    let g = Onestone::debug_state(board,Side::B,1);

    match g.gameover() {
        Some(GameResult::Lose) => {},
        other => panic!("expected Lose for the side about to move, got {:?}",other),
    }
}

#[test]
fn gameover_when_next_side_has_no_pieces() {
    let mut board = [Square::Empty;25];
    board[6] = Square::Piece(Side::A,1);
    // No Side::B pieces remain on the board.

    let g = Onestone::debug_state(board,Side::B,1);

    match g.gameover() {
        Some(GameResult::Lose) => {},
        other => panic!("expected Lose for the side about to move, got {:?}",other),
    }
}

#[test]
fn mcts_finds_a_move_from_the_opening_position() {
    let g = Onestone::with_seed(123);
    let mut mcts = MCTS::new(g);
    mcts.ponder(2000);
    assert!(mcts.best().is_some());
}
