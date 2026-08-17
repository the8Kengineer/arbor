use super::*;
use std::fmt;
use std::fmt::Display;

///Test-only combinatorial game: a shared counter starting at `n`. On each turn the side to move
///subtracts 1, 2, or 3 (never more than remains). Whoever brings the counter to exactly 0 wins.
///This is the classic subtraction game with optimal play trivial to verify by hand: a position is
///losing for the side to move iff `n % 4 == 0` (0 included). That makes it useful for asserting
///MCTS actually converges to the game-theoretically correct move, and for constructing forced
///win/loss positions on demand.
#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Side {A,B}

impl Side {
    pub fn other(&self) -> Side {
        match self {
            Side::A => Side::B,
            Side::B => Side::A,
        }
    }
}

impl Player for Side {}

#[derive(Debug,Copy,Clone,PartialEq)]
pub struct Take(pub u32);

impl Action for Take {}

#[derive(Debug,Copy,Clone)]
pub struct Countdown {
    pub n: u32,
    pub side: Side,
}

impl Display for Countdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"Countdown(n={},side={:?})",self.n,self.side)
    }
}

impl Countdown {
    pub fn new(n: u32) -> Self {
        Countdown {n, side: Side::A}
    }

    ///The game-theoretically optimal outcome for the side to move: true if this is a losing
    ///position (n % 4 == 0), matching the classic subtraction-game (1,2,3) analysis.
    pub fn is_losing_position(&self) -> bool {
        self.n % 4 == 0
    }
}

impl GameState<Side,Take> for Countdown {
    fn actions<F>(&self,f: &mut F) where F: FnMut(Take) {
        debug_assert!(self.gameover().is_none());
        for k in 1..=3 {
            if k <= self.n {
                f(Take(k));
            }
        }
    }

    fn make(&self,action: Take) -> Self {
        Countdown {
            n: self.n - action.0,
            side: self.side.other(),
        }
    }

    fn gameover(&self) -> Option<GameResult> {
        if self.n == 0 {
            //Side to move has nothing to take, meaning the other side just took the last
            //counter and won. Mirrors the "side to play last always wins" pattern used
            //elsewhere in this workspace (e.g. tictactoe).
            Some(GameResult::Lose)
        } else {
            None
        }
    }

    fn player(&self) -> Side {
        self.side
    }
}

///Test-only game that deliberately violates the (unstated but required) `GameState` contract
///that `actions()` must yield at least one action for any state where `gameover()` is `None`.
///`depth == 0` behaves normally (one legal action, leading to `depth == 1`); `depth == 1` yields
///zero actions while still reporting `gameover() == None`. Used only to regression-test the
///guards against this violation - no real game in this workspace ever triggers it.
#[derive(Debug,Copy,Clone,PartialEq)]
pub struct Step;

impl Action for Step {}

#[derive(Debug,Copy,Clone)]
pub struct Broken {
    pub depth: u8,
}

impl Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"Broken(depth={})",self.depth)
    }
}

impl Broken {
    pub fn new() -> Self {
        Broken {depth: 0}
    }
}

impl GameState<Side,Step> for Broken {
    fn actions<F>(&self,f: &mut F) where F: FnMut(Step) {
        if self.depth == 0 {
            f(Step);
        }
        //depth == 1: intentionally zero actions despite gameover() == None
    }

    fn make(&self,_action: Step) -> Self {
        Broken {depth: self.depth + 1}
    }

    fn gameover(&self) -> Option<GameResult> {
        None
    }

    fn player(&self) -> Side {
        Side::A
    }

    fn custom_evaluation(&self) -> f32 {
        0.5
    }
}

#[test]
fn ponder_zero_on_fresh_mcts_is_a_noop() {
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(0);
    assert!(mcts.best().is_none(),"ponder(0) should not have run any real search");
}

#[test]
fn ponder_zero_after_real_search_is_still_a_noop() {
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(50);
    let before = mcts.info.n;
    mcts.ponder(0);
    assert_eq!(before,mcts.info.n,"ponder(0) should not add any iterations");
}
