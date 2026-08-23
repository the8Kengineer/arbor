use std::fmt::Display;
use std::fmt;
use arbor::*;
use lazy_static::lazy_static;
use rand_xorshift::XorShiftRng as Rand;
use rand::{RngCore,SeedableRng};

/// Number of playable (dark) squares on an 8x8 checkers board.
pub const N: usize = 32;

/// Half-moves without a capture or man move before the game is called a draw.
/// Mirrors the spirit of tournament "40-move rule" style draw claims. Without
/// this, two kings could shuffle forever and MCTS random rollouts would never
/// terminate.
pub const NO_PROGRESS_LIMIT: u16 = 80;

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Side {Red, White}

impl Side {
    fn other(&self) -> Self {
        match self {
            Side::Red => Side::White,
            Side::White => Side::Red,
        }
    }

    fn king_row(&self) -> i32 {
        match self {
            Side::Red => 7,
            Side::White => 0,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Red => write!(f,"Red"),
            Side::White => write!(f,"White"),
        }
    }
}

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Kind {Man, King}

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Square {Empty, Occupied(Side,Kind)}

#[derive(Debug,Copy,Clone,PartialEq)]
pub struct Move {pub from: u8, pub to: u8}

/// Convert a playable square index (0..N) to (row,col) on the 8x8 board.
fn rowcol(sq: u8) -> (i32,i32) {
    let row = (sq / 4) as i32;
    let k = (sq % 4) as i32;
    let col = if row % 2 == 0 {2*k + 1} else {2*k};
    (row,col)
}

/// Convert (row,col) to a playable square index, or None if off the board
/// or on a light (unplayable) square.
fn square_at(row: i32, col: i32) -> Option<u8> {
    if !(0..8).contains(&row) || !(0..8).contains(&col) {
        return None;
    }
    if (row + col) % 2 == 0 {
        return None;
    }
    let k = col / 2;
    Some((row*4 + k) as u8)
}

/// Legal diagonal directions (dr,dc) for a piece of the given kind and side.
/// Kings move in all 4 diagonal directions; men move only toward the
/// opponent's edge of the board, even when capturing.
fn directions(kind: Kind, side: Side) -> &'static [(i32,i32)] {
    match kind {
        Kind::King => &[(1,1),(1,-1),(-1,1),(-1,-1)],
        Kind::Man => match side {
            Side::Red => &[(1,1),(1,-1)],
            Side::White => &[(-1,1),(-1,-1)],
        }
    }
}

lazy_static!{
    static ref ZTABLE: [u64; N*4] = {
        let mut table = [0u64; N*4];
        let seed = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
        let mut rand = Rand::from_seed(seed);
        for entry in table.iter_mut() {
            *entry = rand.next_u64();
        }
        table
    };
}
const ZTURN: u64 = 0x123456789ABCDEF0;

fn zindex(sq: u8, side: Side, kind: Kind) -> usize {
    let s = match side {Side::Red => 0, Side::White => 1};
    let k = match kind {Kind::Man => 0, Kind::King => 1};
    (sq as usize)*4 + s*2 + k
}

#[derive(Debug,Copy,Clone)]
pub struct Checkers {
    pub square: [Square; N],
    pub side: Side,
    /// Set to the landing square while a multi-jump chain is in progress.
    /// When set, only further captures from that square are legal, and the
    /// side to move does not change until the chain ends.
    pub jumping: Option<u8>,
    /// Half-moves since the last capture or man move. Used to force a draw
    /// when neither side is making progress (see NO_PROGRESS_LIMIT).
    pub no_progress: u16,
}

impl Display for Checkers {

/*
             Red Turn
  ---------------------------------
7 |   | r |   | r |   | r |   | r |
  ---------------------------------
6 | r |   | r |   | r |   | r |   |
  ---------------------------------
5 |   | r |   | r |   | r |   | r |
  ---------------------------------
4 | - |   | - |   | - |   | - |   |
  ---------------------------------
3 |   | - |   | - |   | - |   | - |
  ---------------------------------
2 | w |   | w |   | w |   | w |   |
  ---------------------------------
1 |   | w |   | w |   | w |   | w |
  ---------------------------------
0 | w |   | w |   | w |   | w |   |
  ---------------------------------
    0   1   2   3   4   5   6   7
*/

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let colnum = "    0   1   2   3   4   5   6   7\n";
        let rowsep = "  ---------------------------------\n";

        let red = self.square.iter().filter(|s| matches!(s,Square::Occupied(Side::Red,_))).count();
        let white = self.square.iter().filter(|s| matches!(s,Square::Occupied(Side::White,_))).count();

        let mut result = String::new();
        result.push_str(&format!("{}",self.side));
        result.push_str(" Turn\n");
        result.push_str(&format!("Red: {}, White: {}\n",red,white));
        result.push_str(rowsep);

        for h in 0..8 {
            let row = 7 - h;
            result.push_str(&format!("{} ",row));
            for col in 0..8 {
                let piece = match square_at(row,col) {
                    Some(sq) => match self.square[sq as usize] {
                        Square::Empty => "-",
                        Square::Occupied(Side::Red,Kind::Man) => "r",
                        Square::Occupied(Side::Red,Kind::King) => "R",
                        Square::Occupied(Side::White,Kind::Man) => "w",
                        Square::Occupied(Side::White,Kind::King) => "W",
                    },
                    None => " ",
                };
                result.push_str(&format!("| {} ",piece));
            }
            result.push_str("|\n");
            result.push_str(rowsep);
        }

        result.push_str(colnum);
        result.push('\n');

        write!(f,"{}",result)
    }
}

impl Checkers {
    pub fn new() -> Self {
        let mut square = [Square::Empty; N];

        for sq in 0..N as u8 {
            let (row,_) = rowcol(sq);
            if row <= 2 {
                square[sq as usize] = Square::Occupied(Side::Red,Kind::Man);
            } else if row >= 5 {
                square[sq as usize] = Square::Occupied(Side::White,Kind::Man);
            }
        }

        Checkers {
            square,
            side: Side::Red,
            jumping: None,
            no_progress: 0,
        }
    }

    #[allow(dead_code)]
    pub fn load(moves: &[Move]) -> Checkers {
        let mut g = Self::new();
        for m in moves {
            g = g.make(*m);
        }
        g
    }

    #[allow(dead_code)]
    pub fn debug_state(square: [Square;N], side: Side) -> Self {
        Checkers {
            square,
            side,
            jumping: None,
            no_progress: 0,
        }
    }

    fn piece_at(&self, sq: u8) -> (Side,Kind) {
        match self.square[sq as usize] {
            Square::Occupied(s,k) => (s,k),
            Square::Empty => panic!("piece_at called on empty square {}",sq),
        }
    }

    /// A simple (non-capturing) move from sq in direction (dr,dc), if legal.
    fn step_move(&self, sq: u8, dr: i32, dc: i32) -> Option<Move> {
        let (row,col) = rowcol(sq);
        let to = square_at(row + dr, col + dc)?;
        if self.square[to as usize] == Square::Empty {
            Some(Move {from: sq, to})
        } else {
            None
        }
    }

    /// A capturing jump from sq in direction (dr,dc), if legal: the adjacent
    /// square must hold an enemy piece, and the square beyond it must be
    /// empty and on the board.
    fn capture_move(&self, sq: u8, side: Side, dr: i32, dc: i32) -> Option<Move> {
        let (row,col) = rowcol(sq);
        let mid = square_at(row + dr, col + dc)?;

        if let Square::Occupied(mside,_) = self.square[mid as usize] {
            if mside != side {
                let (mrow,mcol) = rowcol(mid);
                let to = square_at(mrow + dr, mcol + dc)?;
                if self.square[to as usize] == Square::Empty {
                    return Some(Move {from: sq, to});
                }
            }
        }

        None
    }
}

impl Action for Move {}
impl Player for Side {}

impl GameState<Side,Move> for Checkers {

    fn actions<F>(&self,f: &mut F) where F: FnMut(Move) {
        let mut has_capture = false;

        match self.jumping {
            Some(sq) => {
                let (side,kind) = self.piece_at(sq);
                for &(dr,dc) in directions(kind,side) {
                    if let Some(m) = self.capture_move(sq,side,dr,dc) {
                        f(m);
                        has_capture = true;
                    }
                }
                debug_assert!(has_capture,"mid-jump state must always have a capture available");
            },
            None => {
                for sq in 0..N as u8 {
                    if let Square::Occupied(side,kind) = self.square[sq as usize] {
                        if side != self.side {continue;}
                        for &(dr,dc) in directions(kind,side) {
                            if let Some(m) = self.capture_move(sq,side,dr,dc) {
                                f(m);
                                has_capture = true;
                            }
                        }
                    }
                }

                if !has_capture {
                    for sq in 0..N as u8 {
                        if let Square::Occupied(side,kind) = self.square[sq as usize] {
                            if side != self.side {continue;}
                            for &(dr,dc) in directions(kind,side) {
                                if let Some(m) = self.step_move(sq,dr,dc) {
                                    f(m);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn make(&self,m: Move) -> Self {
        let mut next = *self;

        let (from_row,from_col) = rowcol(m.from);
        let (to_row,to_col) = rowcol(m.to);
        let dr = to_row - from_row;

        let (side,kind) = self.piece_at(m.from);
        debug_assert!(side == self.side,"make called with a piece belonging to the other side");
        debug_assert!(self.square[m.to as usize] == Square::Empty,"make called with an occupied destination");

        next.square[m.from as usize] = Square::Empty;

        let is_jump = dr.abs() == 2;
        if is_jump {
            let mid_row = (from_row + to_row) / 2;
            let mid_col = (from_col + to_col) / 2;
            let mid = square_at(mid_row,mid_col).expect("capture midpoint must be on the board");
            debug_assert!(matches!(self.square[mid as usize], Square::Occupied(s,_) if s != side),"jump did not capture an enemy piece");
            next.square[mid as usize] = Square::Empty;
            next.no_progress = 0;
        } else {
            next.no_progress = if kind == Kind::Man {0} else {self.no_progress + 1};
        }

        let promotes = (kind == Kind::Man) && (to_row == side.king_row());
        let new_kind = if promotes {Kind::King} else {kind};
        next.square[m.to as usize] = Square::Occupied(side,new_kind);

        if promotes {
            next.jumping = None;
            next.side = self.side.other();
        } else if is_jump {
            let further = directions(new_kind,side).iter()
                .any(|&(dr,dc)| next.capture_move(m.to,side,dr,dc).is_some());

            if further {
                next.jumping = Some(m.to);
                next.side = self.side;
            } else {
                next.jumping = None;
                next.side = self.side.other();
            }
        } else {
            next.jumping = None;
            next.side = self.side.other();
        }

        next
    }

    fn gameover(&self) -> Option<GameResult> {
        if self.no_progress >= NO_PROGRESS_LIMIT {
            return Some(GameResult::Draw);
        }

        let mut any = false;
        self.actions(&mut |_| any = true);

        if any {
            None
        } else {
            Some(GameResult::Lose)
        }
    }

    fn hash(&self) -> u64 {
        let mut h = 0;
        for sq in 0..N as u8 {
            if let Square::Occupied(side,kind) = self.square[sq as usize] {
                h ^= ZTABLE[zindex(sq,side,kind)];
            }
        }
        if self.side == Side::White {
            h ^= ZTURN;
        }
        h
    }

    fn player(&self) -> Side {
        self.side
    }
}
