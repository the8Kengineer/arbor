use std::fmt::Display;
use std::fmt;
use arbor::*;

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Side {A,B}

impl Side {
    fn other(&self) -> Self {
        match *self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::A => write!(f,"A"),
            Side::B => write!(f,"B"),
        }
    }
}

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Square {
    Empty,
    Piece(Side,u8),
}

///A move designates which numbered cube to relocate and its destination square (row-major index into the 5x5 board). `number == 0` is a reserved sentinel used only for the (practically unreachable) case where a side has no legal moves at all; it passes the turn without changing the board.
#[derive(Debug,Copy,Clone,PartialEq)]
pub struct Move {
    pub number: u8,
    pub to: usize,
}

const PASS: Move = Move {number: 0, to: usize::MAX};

const SIZE: usize = 5;
const SPACES: usize = SIZE*SIZE;

#[inline]
fn rowcol(i: usize) -> (isize,isize) {
    ((i/SIZE) as isize,(i%SIZE) as isize)
}

#[inline]
fn index(row: isize, col: isize) -> usize {
    (row as usize)*SIZE + (col as usize)
}

//Splitmix64-style step, mixing the chosen action into the seed so the resulting die sequence
//depends on the path of play, not merely the ply depth from the root. This matters because
//arbor's search replays "make" from the root on every tree visit (see arbor/src/search.rs); if
//the seed transition only depended on the prior seed, every node at a given depth would derive
//the same die roll no matter which moves led there, letting the search "know" future rolls in
//advance.
#[inline]
fn next_die(seed: u64, action: Move) -> (u64,u8) {
    let mixed = seed
        ^ (action.number as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (action.to as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);

    let mut z = mixed.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;

    let die = ((z % 6) as u8) + 1;
    (z,die)
}

fn random_seed() -> u64 {
    let mut buf = [0u8;8];
    getrandom::getrandom(&mut buf).expect("getrandom failed to produce a seed");
    u64::from_le_bytes(buf)
}

///The starting layout for each side, given as (board index, cube number) pairs. Side A occupies
///the top-left corner and advances right/down/down-right; side B occupies the bottom-right
///corner (mirrored so that opposing corners hold matching numbers) and advances
///left/up/up-left.
const START_A: [(usize,u8);6] = [(0,1),(1,2),(2,4),(5,3),(6,5),(10,6)];
const START_B: [(usize,u8);6] = [(14,6),(18,5),(19,3),(22,4),(23,2),(24,1)];

#[derive(Debug,Copy,Clone)]
pub struct Onestone {
    pub board: [Square;SPACES],
    pub side: Side,
    pub die: u8,
    seed: u64,
}

impl Onestone {
    pub fn new() -> Self {
        Self::with_seed(random_seed())
    }

    pub fn with_seed(seed: u64) -> Self {
        let mut board = [Square::Empty;SPACES];

        for &(i,n) in START_A.iter() {
            board[i] = Square::Piece(Side::A,n);
        }
        for &(i,n) in START_B.iter() {
            board[i] = Square::Piece(Side::B,n);
        }

        //Prime the first die roll. There is no "prior action" yet, so mix in a fixed marker
        //instead of an actual Move.
        let (seed,die) = next_die(seed,Move {number: 0, to: 0});

        Onestone {board, side: Side::A, die, seed}
    }

    ///Construct an arbitrary board position directly, bypassing setup and legality checks.
    ///Intended for unit tests that need to exercise a specific mid-game position.
    #[allow(dead_code)]
    pub fn debug_state(board: [Square;SPACES], side: Side, die: u8) -> Self {
        Onestone {board, side, die, seed: 0}
    }

    #[allow(dead_code)]
    pub fn load(seed: u64, moves: &[Move]) -> Onestone {
        let mut g = Onestone::with_seed(seed);
        for m in moves {
            g = g.make(*m);
        }
        g
    }

    fn find(&self, side: Side, number: u8) -> Option<usize> {
        for (i,sq) in self.board.iter().enumerate() {
            if *sq == Square::Piece(side,number) {
                return Some(i);
            }
        }
        None
    }

    fn present(&self, side: Side) -> [bool;7] {
        let mut p = [false;7];
        for sq in self.board.iter() {
            if let Square::Piece(s,n) = sq {
                if *s == side {
                    p[*n as usize] = true;
                }
            }
        }
        p
    }

    fn deltas(side: Side) -> [(isize,isize);3] {
        match side {
            Side::A => [(0,1),(1,0),(1,1)],
            Side::B => [(0,-1),(-1,0),(-1,-1)],
        }
    }

    fn emit_moves_for<F>(&self, number: u8, f: &mut F) -> usize where F: FnMut(Move) {
        let side = self.side;
        let mut count = 0;

        if let Some(pos) = self.find(side,number) {
            let (row,col) = rowcol(pos);

            for (dr,dc) in Self::deltas(side).iter() {
                let r = row + dr;
                let c = col + dc;

                if (0..SIZE as isize).contains(&r) && (0..SIZE as isize).contains(&c) {
                    let to = index(r,c);
                    let blocked = matches!(self.board[to], Square::Piece(s,_) if s == side);

                    if !blocked {
                        f(Move {number, to});
                        count += 1;
                    }
                }
            }
        }

        count
    }

    fn target_corner(side: Side) -> usize {
        match side {
            Side::A => SPACES - 1,
            Side::B => 0,
        }
    }
}

impl Default for Onestone {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Onestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f,"Side {} to play, die: {}",self.side,self.die)?;
        writeln!(f,"-----------------------------")?;

        for row in 0..SIZE {
            write!(f,"|")?;
            for col in 0..SIZE {
                match self.board[index(row as isize,col as isize)] {
                    Square::Empty => write!(f,"     |")?,
                    Square::Piece(Side::A,n) => write!(f," [{}] |",n)?,
                    Square::Piece(Side::B,n) => write!(f," ({}) |",n)?,
                }
            }
            writeln!(f)?;
            writeln!(f,"-----------------------------")?;
        }

        Ok(())
    }
}

impl Action for Move {}
impl Player for Side {}

impl GameState<Side,Move> for Onestone {

    fn actions<F>(&self,f: &mut F) where F: FnMut(Move) {
        debug_assert!(self.gameover().is_none());

        let side = self.side;
        let present = self.present(side);

        let mut candidates: [u8;2] = [0,0];
        let mut nc = 0;

        if present[self.die as usize] {
            candidates[0] = self.die;
            nc = 1;
        } else {
            let mut lower = self.die as i32 - 1;
            while lower >= 1 {
                if present[lower as usize] {
                    candidates[nc] = lower as u8;
                    nc += 1;
                    break;
                }
                lower -= 1;
            }

            let mut higher = self.die as i32 + 1;
            while higher <= 6 {
                if present[higher as usize] {
                    candidates[nc] = higher as u8;
                    nc += 1;
                    break;
                }
                higher += 1;
            }
        }

        let mut count = 0;
        for &n in candidates[0..nc].iter() {
            count += self.emit_moves_for(n,f);
        }

        if count == 0 {
            for n in 1..=6u8 {
                if present[n as usize] {
                    count += self.emit_moves_for(n,f);
                }
            }
        }

        if count == 0 {
            f(PASS);
        }
    }

    fn make(&self,action: Move) -> Self {
        let side = self.side;
        let mut board = self.board;

        if action.number != 0 {
            let from = self.find(side,action.number)
                .expect("make called with a move for a piece that isn't on the board");
            board[from] = Square::Empty;
            board[action.to] = Square::Piece(side,action.number);
        }

        let (seed,die) = next_die(self.seed,action);

        Onestone {
            board,
            side: side.other(),
            die,
            seed,
        }
    }

    fn gameover(&self) -> Option<GameResult> {
        let mover = self.side.other();

        let reached = matches!(
            self.board[Self::target_corner(mover)],
            Square::Piece(s,_) if s == mover
        );

        let next_to_move_pieces = self.board.iter()
            .filter(|sq| matches!(sq, Square::Piece(s,_) if *s == self.side))
            .count();

        if reached || next_to_move_pieces == 0 {
            //Mirrors the "side to play last always wins" pattern used by tictactoe: the mover
            //who just acted won, so the side about to move (self.side) has lost.
            Some(GameResult::Lose)
        } else {
            None
        }
    }

    fn player(&self) -> Side {
        self.side
    }

    //Transposition is intentionally not implemented. Two states with identical board/side/die
    //but different `seed` values are not interchangeable: their future die rolls diverge, so
    //treating them as the same node (as MCTS transposition would) is not correct here.
}
