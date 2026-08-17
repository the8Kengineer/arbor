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
#[should_panic(expected = "zero actions")]
fn expansion_panics_clearly_on_zero_actions_state() {
    //With custom_evaluation enabled, the first visit to a Leaf never calls actions() (it calls
    //custom_evaluation() instead), so Broken's depth==1 state survives its first visit
    //undetected and only gets its actions() called when the *second* visit triggers expansion -
    //isolating this regression test to the expansion code path specifically, rather than
    //rollout's (see the next test).
    let mut mcts = MCTS::new(Broken::new()).with_custom_evaluation();
    mcts.ponder(2);
}

#[test]
#[should_panic(expected = "zero actions")]
fn rollout_panics_clearly_on_zero_actions_state() {
    //Without custom_evaluation, the root's forced first expansion immediately recurses into its
    //one real child (depth 1), whose first visit rolls out - hitting Broken's depth==1 state
    //inside rollout()'s own loop, isolating this regression test to the rollout code path.
    let mut mcts = MCTS::new(Broken::new());
    mcts.ponder(1);
}

#[test]
fn ponder_on_gameover_root_does_not_panic() {
    let mut mcts = MCTS::new(Countdown::new(0));
    mcts.ponder(10);
    assert!(mcts.best().is_none(),"there is no move to make from an already-decided position");
}

#[test]
fn ponder_on_gameover_root_is_repeatable() {
    //Calling ponder() again on an already-decided root should keep returning cleanly, not just
    //on the very first call.
    let mut mcts = MCTS::new(Countdown::new(0));
    mcts.ponder(1);
    mcts.ponder(100);
    assert!(mcts.best().is_none());
}

#[test]
fn best_prefers_visit_count_over_lucky_average() {
    //Bootstrap a root with 3 real children (Take(1), Take(2), Take(3)) via a single iteration,
    //then hand-craft their statistics to isolate the exact selection mechanism: Take(1) looks
    //*better* by raw average (a single lucky rollout) but is barely explored; Take(2) has a
    //lower average but is thoroughly explored. Whether best() picks the well-explored move over
    //the lucky-but-unreliable one is exactly the behavior under test - unreachable by only
    //asserting the final move is "good", since with enough iterations both old and new
    //selection converge to the same answer anyway.
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(1);

    match mcts.stack[0] {
        Node::Branch(_,_,player,_,_,_) => assert_eq!(player,Side::A),
        _ => panic!("expected root to be a branch after one iteration"),
    }

    //Node fields are (has_sibling, action, player, value-sum, visits[, child]). Values are
    //stored from the child's own player's perspective (Side::B here), so a move that *looks*
    //strong for the root (Side::A) needs a *low* stored average.
    mcts.stack[1] = Node::Leaf(true,Take(1),Side::B,0.05,1);   // root-perspective avg 0.95, n=1
    mcts.stack[2] = Node::Leaf(true,Take(2),Side::B,20.0,50);  // root-perspective avg 0.60, n=50
    mcts.stack[3] = Node::Leaf(false,Take(3),Side::B,25.0,50); // root-perspective avg 0.50, n=50

    assert_eq!(mcts.best(),Some(Take(2)),"the well-explored move should win over the lucky, barely-visited one");
}

#[test]
fn best_prefers_proven_win_over_a_much_better_explored_uncertain_move() {
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(1);

    mcts.stack[1] = Node::Terminal(true,Take(1),Side::B,0.0);   // proven win for the root (Side::A)
    mcts.stack[2] = Node::Leaf(true,Take(2),Side::B,5.0,50);    // root-perspective avg 0.90, n=50
    mcts.stack[3] = Node::Leaf(false,Take(3),Side::B,25.0,50);  // root-perspective avg 0.50, n=50

    assert_eq!(mcts.best(),Some(Take(1)),"a proven win must be chosen over any merely-probable move");
}

#[test]
fn best_converges_to_the_game_theoretically_correct_move() {
    //n=6: taking 2 leaves the opponent at n=4, a losing position for them (4 % 4 == 0) - the
    //unique correct move. An end-to-end convergence check, complementing the two precise tests
    //above.
    let mut mcts = MCTS::new(Countdown::new(6));
    mcts.ponder(20000);
    assert_eq!(mcts.best(),Some(Take(2)));
}

#[test]
fn ponder_zero_after_real_search_is_still_a_noop() {
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(50);
    let before = mcts.info.n;
    mcts.ponder(0);
    assert_eq!(before,mcts.info.n,"ponder(0) should not add any iterations");
}
