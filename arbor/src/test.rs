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
fn best_prefers_any_unresolved_move_over_a_proven_loss() {
    //A proven loss must rank below an unresolved move, however mediocre that move's average
    //looks so far - it might still turn out fine, whereas the proven loss provably won't. This
    //regression-tests a real bug: an earlier version of best() gave every Terminal node (loss
    //included) a "treat as infinite visit count" sentinel meant only for proven wins, which let
    //a proven loss's exact value (0.0) beat a perfectly reasonable but merely-average
    //unresolved move on the visit-count comparison, in effect always steering into a known loss
    //whenever one was available among the candidates.
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(1);

    mcts.stack[1] = Node::Terminal(true,Take(1),Side::B,1.0);  // proven loss for the root (Side::A)
    mcts.stack[2] = Node::Leaf(true,Take(2),Side::B,20.0,50);  // root-perspective avg 0.60, n=50
    mcts.stack[3] = Node::Leaf(false,Take(3),Side::B,49.0,50); // root-perspective avg 0.02, n=50

    assert_eq!(mcts.best(),Some(Take(2)),"any unresolved move should be preferred over a proven loss");
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
fn uct_ties_are_broken_randomly_not_always_first_sibling() {
    //On the very first real iteration, all 3 children of the root start tied at
    //f32::INFINITY (none have been tried yet) - the most visible case of a UCT tie. Which one
    //gets explored first should vary across independently-seeded searches; before reservoir
    //sampling it was always whichever action Countdown::actions() enumerates first (Take(1)).
    let mut counts = [0u32;3];

    for _ in 0..40 {
        let mut mcts = MCTS::new(Countdown::new(10)).with_entropy();
        mcts.ponder(1);

        let c = match mcts.stack[0] {
            Node::Branch(_,_,_,_,_,c) => c,
            _ => panic!("expected root to be a branch after one iteration"),
        };

        for offset in 0..3 {
            if let Node::Leaf(_,Take(k),_,_,_) = mcts.stack[c + offset] {
                counts[(k - 1) as usize] += 1;
                break;
            }
        }
    }

    let distinct = counts.iter().filter(|&&n| n > 0).count();
    assert!(distinct > 1,"expected tie-breaking to vary which child is explored first across 40 runs, got counts {:?}",counts);
}

#[test]
fn f64_accumulation_resists_the_drift_f32_would_suffer_at_high_visit_counts() {
    //A running value-sum accumulated as f32 has only ~7 significant decimal digits; summing a
    //value that isn't exactly representable (like 1/3) millions of times compounds rounding
    //error at every single addition. f64's error per step is around 9 orders of magnitude
    //smaller, so the same number of additions leaves it comparatively exact. This is the
    //general mathematical fact motivating widening Node::Leaf/Branch's `w` field to f64 (see
    //arbor/src/lib.rs) - demonstrated directly here rather than by driving an actual multi-
    //million-iteration search, which would make this test slow without testing anything more
    //specific than the arithmetic itself.
    let iterations: u32 = 5_000_000;
    let increment = 1.0/3.0;

    let mut sum32: f32 = 0.0;
    let mut sum64: f64 = 0.0;
    for _ in 0..iterations {
        sum32 += increment as f32;
        sum64 += increment;
    }

    let expected = iterations as f64 * increment;
    let err32 = (sum32 as f64 - expected).abs();
    let err64 = (sum64 - expected).abs();

    assert!(err64 < 1.0,"f64 accumulation error should stay tiny at {} iterations, got {}",iterations,err64);
    assert!(err32 > err64 * 1000.0,"expected f32 accumulation to drift dramatically more than f64 at {} iterations (f32 err={}, f64 err={})",iterations,err32,err64);
}

#[test]
fn leaf_value_sum_stores_high_visit_counts_without_losing_precision() {
    //16_777_217.0 is the smallest positive integer f32 cannot represent (2^24 + 1) - a sum an
    //f32 accumulator could plausibly reach at a heavily-visited node (e.g. the root) over a
    //long-running search. Confirm the tree actually stores and reports it exactly.
    let n: u32 = 16_777_217 * 2;
    let w: f64 = 16_777_217.0;

    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(1);
    let c = match mcts.stack[0] {
        Node::Branch(_,_,_,_,_,c) => c,
        _ => panic!("expected root to be a branch after one iteration"),
    };
    //Overwrite whichever child was explored first with a synthetic, precision-sensitive value;
    //Side::B is the mover at any child of a Side::A root, matching what a real search would use.
    mcts.stack[c] = Node::Leaf(false,Take(1),Side::B,w,n);

    let mut reported = Vec::new();
    mcts.ply(&mut |(a,avg,_e)| reported.push((a,avg)));
    let (_,avg) = reported.into_iter().find(|(a,_)| *a == Take(1)).expect("Take(1) should be reported");

    assert!((avg - 0.5).abs() < 1e-6,"expected the average to remain exactly 0.5, got {}",avg);
}

#[test]
fn solver_proves_a_forced_loss_and_stops_further_work() {
    //n=4 is a losing position for the side to move (4 % 4 == 0): every reply (take 1, 2, or 3)
    //leaves the opponent at n=3, n=2, or n=1 - all winning positions for them. A tiny, shallow
    //game tree, well within reach of full solving.
    let mut mcts = MCTS::new(Countdown::new(4));
    mcts.ponder(5000);

    match mcts.stack[0] {
        Node::Terminal(_,_,player,w) => {
            assert_eq!(player,Side::A);
            assert_eq!(w,0.0,"a forced loss should be proven with certainty (0.0), not just estimated");
        },
        _ => panic!("expected the root to be fully solved as Terminal after 5000 iterations of a 3-ply-deep forced loss"),
    }

    let n_before = mcts.info.n;
    mcts.ponder(1000);
    assert_eq!(mcts.info.n,n_before,"once solved, further ponder() calls should do no additional work");

    //Every move loses equally, but best() must still recommend one - solving shouldn't leave
    //the caller with no legal move to actually play.
    assert!(mcts.best().is_some());
}

#[test]
fn solver_proves_a_forced_win_via_a_single_winning_child() {
    //n=5: taking 1 leaves the opponent at the forced-loss position n=4 - the unique winning
    //reply. Confirms the "OR node" half of the solver: one proven-winning child is enough to
    //solve the branch, without needing every other child resolved too.
    let mut mcts = MCTS::new(Countdown::new(5));
    mcts.ponder(20000);

    match mcts.stack[0] {
        Node::Terminal(_,a,player,w) => {
            assert_eq!(player,Side::A);
            assert_eq!(w,1.0,"a forced win should be proven with certainty (1.0)");
            assert_eq!(a,Take(1),"the remembered move should be the actual winning one");
        },
        _ => panic!("expected the root to be fully solved as Terminal after 20000 iterations"),
    }

    assert_eq!(mcts.best(),Some(Take(1)));
}

#[test]
fn ponder_zero_after_real_search_is_still_a_noop() {
    let mut mcts = MCTS::new(Countdown::new(10));
    mcts.ponder(50);
    let before = mcts.info.n;
    mcts.ponder(0);
    assert_eq!(before,mcts.info.n,"ponder(0) should not add any iterations");
}
