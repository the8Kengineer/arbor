use super::*;
use rand::SeedableRng;
use rand::RngCore;
use std::collections::VecDeque;

///RAVE's equivalence parameter (k in the standard beta = sqrt(k / (3n + k)) blend): roughly how
///many real visits a node needs before its RAVE estimate stops mattering much. 1000 is a
///commonly cited default in the RAVE literature (Gelly & Silver).
const RAVE_EQUIVALENCE: f64 = 1000.0;

impl GameResult {
    #[inline]
    fn value(&self) -> f32 {
        match *self {
            GameResult::Win => 1.0,
            GameResult::Lose => 0.0,
            GameResult::Draw => 0.5,
        }
    }
}

impl<P: Player, A: Action, S: GameState<P,A>> MCTS<P, A, S> {
    ///Call this method to instantiate a new search with default parameters. The root game state from which to search is passed as a value to be owned by the MCTS struct.
    pub fn new(root: S) -> Self {
        let s = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
        Self {
            exploration: 2.0f32.sqrt(),
            expansion: 0,
            use_custom_evaluation: false,
            use_transposition: false,
            use_rave: false,
            info: Info::default(),
            root: root,
            stack: Vec::new(),
            actions: Vec::new(),
            rand: Rng::from_seed(s),
            map: HashMap::default(),
            rave: Vec::new(),
            played: Vec::new(),
        }
    }

    ///Pick the best move after some time spent pondering. Returns None if ponder has not yet been called, or if the root game state is already a game-over position.
    ///
    ///Selection ranks candidates into four tiers, highest first: a proven win, a proven draw (or other non-extreme proven value), an unresolved child, then a proven loss. Within the unresolved tier, the most-visited child ("robust child") is preferred, using average value only as a tiebreaker - visit count is a far more reliable indicator than raw average value, which a lightly-visited child can reach purely by a lucky early rollout. Proven values, by construction, are never that lucky, which is also why a proven loss still ranks below every unresolved child instead of being treated as simply "the worst average": an unresolved child might turn out fine, a proven loss provably won't.
    ///
    ///If the search fully solves the position (see MCTS-Solver in go()), the root itself becomes Terminal and this returns the move that was proven best, same as always.
    pub fn best(&self) -> Option<A> {
        if self.stack.len() == 0 {
            return None;
        }

        let (player,c) = match self.stack[0] {
            Node::Branch(_,_,player,_,_,c) => (player,c),
            Node::Terminal(_,a,_,_) => return Some(a),
            _ => return None,
        };

        let mut best: Option<A> = None;
        let mut best_tier: u8 = 0;
        let mut best_n: u32 = 0;
        let mut best_avg: f32 = -1.0;

        let mut sibling = Some(c);
        while let Some(u) = sibling {
            let (s,candidate,tier,n,avg) = match self.stack[u] {
                Node::Terminal(s,a,p,w) => {
                    let w = if p == player {w} else {1.0 - w};
                    let tier = if w >= 1.0 {3} else if w <= 0.0 {0} else {2};
                    (s,a,tier,0,w)
                },
                Node::Leaf(s,a,p,w,n) |
                Node::Branch(s,a,p,w,n,_) => {
                    let avg = (w/(n as f64)) as f32;
                    let avg = if p == player {avg} else {1.0 - avg};
                    (s,a,1u8,n,avg)
                },
                Node::Unknown(s,a) => (s,a,1u8,0,0.5),
                Node::Transpose(_,_,_) =>
                    panic!("Transpositions should not be possible at root ply"),
            };

            let better = if tier != best_tier {
                tier > best_tier
            } else if tier == 1 {
                n > best_n || (n == best_n && avg > best_avg)
            } else {
                avg > best_avg
            };

            if best.is_none() || better {
                best = Some(candidate);
                best_tier = tier;
                best_n = n;
                best_avg = avg;
            }

            sibling = s.then(||u+1);
        }

        best
    }

    ///Iterate through the actions in the first ply. The callback f is called for each action in the first ply with a tuple of (a, w, s) where a is the action, w is the expected value of the action, and s is the confidence in the value of the action. s is similar to standard deviation where closer to zero is more confident.
    pub fn ply<F>(&self, f: &mut F) where F: FnMut((A,f32,f32)) {
        if self.stack.len() == 0 {
            return;
        }

        if let Node::Branch(_,_,player,_,_,c) = self.stack[0] {
            let mut sibling = Some(c);
            while let Some(u) = sibling {
                match self.stack[u] {
                    Node::Leaf(s,a,p,w,n) |
                    Node::Branch(s,a,p,w,n,_) => {
                        let nf = n as f64;
                        let w = w/nf;
                        let w = if p == player {w} else {1.0 - w};
                        let e = (0.5/nf + (w*(1.0 - w)/nf).sqrt()) as f32;
                        f((a,w as f32,e));
                        sibling = s.then(||u+1);
                    },
                    Node::Terminal(s,a,p,w) => {
                        let w = if p == player {w} else {1.0 - w};
                        f((a,w,0.0));
                        sibling = s.then(||u+1);
                    },
                    Node::Unknown(s,a) => {
                        f((a,0.5,0.5));
                        sibling = s.then(||u+1);
                    },
                    Node::Transpose(_,_,_) => 
                        panic!("Transpositions should not be possible at root ply")
                }
            }
        } else {
            debug_assert!(false,"root node should not be a branch");
        }
    }
    
    ///Call this method to search the root game state a given number of iterations. This method may be called any number of times to improve the search results. Call ply or best to get the current search results.
    pub fn ponder(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        if self.stack.len() == 0 {
            if self.root.gameover().is_some() {
                //Nothing to search from an already-decided position. Leave the stack empty so
                //ply()/best() report "no move" the same way they do before ponder() is ever
                //called, rather than panicking trying to bootstrap a root with no actions.
                return;
            }

            let mut actions = Vec::new();
            self.root.actions(&mut |a| actions.push(a));


            self.stack.push(Node::Leaf(
                false,
                // This action is never used, so it doesn't matter what it is
                *actions.first().expect("should have at least one action"),
                self.root.player(),
                0.5,
                1
            ));
            if self.use_rave {
                self.rave.push((0,0.0));
            }

            self.info.leaf = 1;
            
            //Call go once with expansion set to zero to force the root to expand
            let root = self.root;
            let expansion = self.expansion;
            self.expansion = 0;
            self.go(&root, 0);
            self.expansion = expansion;
            if self.use_rave {
                self.played.clear();
            }
            self.ponder(n - 1);
        } else {
            let root = self.root;
            for _ in 0..n {
                self.go(&root,0);
                if self.use_rave {
                    self.played.clear();
                }
            }
            
            self.info.bytes = self.stack.len() * std::mem::size_of::<Node<P,A>>();
        }
    }

    ///Advance the search onto the state that results from playing `action` at the root, reusing
    ///whatever was already explored under that specific move instead of discarding the whole
    ///tree the way constructing a fresh `MCTS::new()` after every real move would. This is
    ///normally the biggest wasted-work opportunity across a full game: the child of the move
    ///actually played is, by construction, the most-explored part of the entire tree.
    ///
    ///Search configuration (exploration constant, expansion minimum, custom evaluation,
    ///transposition, and RNG state) carries over unchanged. If there's nothing worth reusing -
    ///`ponder` was never called, `action` wasn't among the root's children, or that child was
    ///never actually explored (still `Unknown`) or was already solved to a bare `Terminal`
    ///(which no longer carries a pointer to its own children to reuse, and must not be kept as
    ///the new root regardless - see the comment inline) - this behaves like starting fresh from
    ///the resulting state.
    pub fn advance(mut self, action: A) -> Self {
        let reusable = match self.stack.get(0) {
            Some(&Node::Branch(_,_,_,_,_,c)) => {
                let mut sibling = Some(c);
                let mut found = None;
                while let Some(u) = sibling {
                    let (matches,has_sibling) = match self.stack[u] {
                        Node::Unknown(s,a) => (a == action,s),
                        Node::Terminal(s,a,_,_) => (a == action,s),
                        Node::Leaf(s,a,_,_,_) => (a == action,s),
                        Node::Branch(s,a,_,_,_,_) => (a == action,s),
                        Node::Transpose(s,a,_) => (a == action,s),
                    };
                    if matches {
                        found = Some(u);
                        break;
                    }
                    sibling = has_sibling.then(||u+1);
                }
                //Only a Branch is worth relocating as the new root. A Terminal specifically must
                //not be kept: its `a` field means "how my old parent reached me", not "the best
                //move from here" (only index 0 ever gets that meaning rewritten, in go()'s
                //solver step) - and since ponder() only ever re-bootstraps an *empty* stack,
                //keeping a lone stale Terminal as root would permanently stop all further search
                //while best() kept reporting a meaningless action forever.
                found.filter(|&u| matches!(self.stack[u],Node::Branch(_,_,_,_,_,_)))
            },
            _ => None,
        };

        self.root = self.root.make(action);

        match reusable {
            Some(old_root) => self.relocate(old_root),
            None => {
                self.stack.clear();
                self.map.clear();
                self.info = Info::default();
                self.rave.clear();
                self.played.clear();
            }
        }

        self
    }

    ///Compact the subtree rooted at `old_root` (an index into the *current* self.stack) into a
    ///fresh Vec starting at index 0, discarding everything else, and remapping every internal
    ///index (a Branch's child-start `c`) from old to new. Nodes are copied in level order via a
    ///queue that enqueues each Branch's *entire* sibling run atomically, which keeps every
    ///Branch's children contiguous in the new Vec too - required, since sibling traversal
    ///elsewhere just walks `index + 1` rather than following an explicit next-pointer.
    ///
    ///Transpose nodes are not followed (their target may not even be part of this subtree - it
    ///could point at a sibling's subtree being discarded, or an ancestor) and are conservatively
    ///reset to Unknown instead: the position gets re-explored and, if applicable, re-transposed
    ///next time it's reached, rather than risking a dangling or contiguity-violating reference.
    fn relocate(&mut self,old_root: usize) {
        let mut order: Vec<usize> = vec![old_root];
        let mut old_to_new: HashMap<usize,usize> = HashMap::default();
        old_to_new.insert(old_root,0);

        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(old_root);

        while let Some(old_index) = queue.pop_front() {
            if let Node::Branch(_,_,_,_,_,c) = self.stack[old_index] {
                let mut sibling = Some(c);
                while let Some(u) = sibling {
                    let new_index = order.len();
                    old_to_new.insert(u,new_index);
                    order.push(u);
                    queue.push_back(u);
                    sibling = self.sibling_flag(u).then(||u+1);
                }
            }
        }

        let mut new_stack: Vec<Node<P,A>> = Vec::with_capacity(order.len());
        let mut new_rave: Vec<(u32,f64)> = Vec::with_capacity(if self.use_rave {order.len()} else {0});
        let mut new_info = Info::default();

        for &old_index in &order {
            let node = match self.stack[old_index] {
                Node::Unknown(s,a) => {
                    new_info.unknown += 1;
                    Node::Unknown(s,a)
                },
                Node::Terminal(s,a,p,w) => {
                    new_info.terminal += 1;
                    Node::Terminal(s,a,p,w)
                },
                Node::Leaf(s,a,p,w,n) => {
                    new_info.leaf += 1;
                    Node::Leaf(s,a,p,w,n)
                },
                Node::Branch(s,a,p,w,n,c) => {
                    new_info.branch += 1;
                    Node::Branch(s,a,p,w,n,old_to_new[&c])
                },
                Node::Transpose(s,a,_) => {
                    new_info.unknown += 1;
                    Node::Unknown(s,a)
                },
            };
            new_stack.push(node);
            //Kept in lockstep with new_stack so RAVE data survives relocation for reused nodes,
            //same as their ordinary visit statistics; a relocated Transpose (now Unknown, see
            //above) has no RAVE data of its own to carry over, so it gets a fresh zero entry.
            if self.use_rave {
                let entry = if matches!(self.stack[old_index],Node::Transpose(_,_,_)) {
                    (0,0.0)
                } else {
                    self.rave[old_index]
                };
                new_rave.push(entry);
            }
        }

        new_info.bytes = new_stack.len() * std::mem::size_of::<Node<P,A>>();
        if let Node::Branch(_,_,_,w,n,_) = new_stack[0] {
            new_info.n = n;
            new_info.q = (w/(n as f64)) as f32;
        }

        self.stack = new_stack;
        self.map.clear();
        self.info = new_info;
        if self.use_rave {
            self.rave = new_rave;
        }
        self.played.clear();
    }

    fn sibling_flag(&self,index: usize) -> bool {
        match self.stack[index] {
            Node::Unknown(s,_) => s,
            Node::Terminal(s,_,_,_) => s,
            Node::Leaf(s,_,_,_,_) => s,
            Node::Branch(s,_,_,_,_,_) => s,
            Node::Transpose(s,_,_) => s,
        }
    }

    ///MCTS-Solver: check whether a branch's siblings (starting at `c`) now prove the branch's
    ///own value, from `player`'s (the branch's own mover's) perspective. A single proven win is
    ///enough (an "OR" node - the mover just takes it), so that short-circuits immediately even
    ///if other children are still unresolved. A proven loss requires every child to already be
    ///a proven loss - if even one sibling isn't yet a proven Terminal, nothing can be concluded,
    ///since that sibling could still turn out to be the winning move.
    ///
    ///Transpose siblings are conservatively treated as unresolved rather than following the
    ///pointer to check the transposed target, since transposition is already documented as
    ///experimental; this can delay a solve but never produces an incorrect one.
    fn solved_value(&self,c: usize,player: P) -> Option<f32> {
        let mut sibling = Some(c);
        let mut all_are_losses = true;

        while let Some(u) = sibling {
            match self.stack[u] {
                Node::Terminal(_,_,p,w) => {
                    let w = if p == player {w} else {1.0 - w};
                    if w >= 1.0 {
                        return Some(1.0);
                    }
                    if w > 0.0 {
                        all_are_losses = false;
                    }
                },
                _ => {
                    all_are_losses = false;
                }
            }
            sibling = self.sibling_flag(u).then(||u+1);
        }

        if all_are_losses {Some(0.0)} else {None}
    }

    ///Blend the RAVE/AMAF estimate stored at `index` into `avg` (the ordinary per-visit
    ///average), per the standard beta = sqrt(k / (3n + k)) schedule: beta starts near 1 (RAVE
    ///dominates while a node has few real visits) and fades toward 0 as `n` grows (a node's own
    ///accumulated visits are always more trustworthy than the coarser AMAF estimate once there
    ///are enough of them). A no-op when RAVE is disabled or `index` has no RAVE visits yet.
    fn rave_blend(&self,index: usize,p: P,player: P,avg: f32,nf: f64) -> f32 {
        if !self.use_rave {
            return avg;
        }
        let (rn,rw) = self.rave[index];
        if rn == 0 {
            return avg;
        }
        let rnf = rn as f64;
        let rw = if p == player {rw} else {rnf - rw};
        let ravg = (rw/rnf) as f32;
        let beta = (RAVE_EQUIVALENCE / (3.0*nf + RAVE_EQUIVALENCE)).sqrt() as f32;
        (1.0 - beta)*avg + beta*ravg
    }

    ///RAVE/AMAF credit pass: for every Leaf/Branch sibling in this branch's child list (starting
    ///at `c`) whose own action was played anywhere in the rest of this simulation
    ///(`self.played[start..]`, which covers both further tree selections below this point and
    ///the eventual rollout), update its RAVE statistics as if it had been the one actually
    ///chosen this visit - not just the sibling that really was. `v` is already `player`'s (this
    ///branch's own mover's) perspective; each sibling's stored `p` may differ (a child's mover
    ///need not be every game's strict opponent), so it's re-flipped per sibling exactly like an
    ///ordinary visit would be.
    ///
    ///Only Leaf/Branch siblings participate: Unknown has no stored player to flip against
    ///(assuming strict alternation here would be wrong for games where it doesn't hold), and
    ///Terminal's proven value is never blended with a noisier AMAF estimate in the first place
    ///(see rave_blend), so crediting it would only ever go unused.
    fn rave_update(&mut self,c: usize,player: P,v: f32,start: usize) {
        let relevant: Vec<A> = self.played[start..].to_vec();

        let mut sibling = Some(c);
        while let Some(u) = sibling {
            let has_sibling = match self.stack[u] {
                Node::Leaf(s,a,p,_,_) |
                Node::Branch(s,a,p,_,_,_) => {
                    if relevant.contains(&a) {
                        let credit = if p == player {v as f64} else {1.0 - v as f64};
                        let (rn,rw) = self.rave[u];
                        self.rave[u] = (rn + 1,rw + credit);
                    }
                    s
                },
                Node::Terminal(s,_,_,_) => s,
                Node::Unknown(s,_) => s,
                Node::Transpose(s,_,_) => s,
            };
            sibling = has_sibling.then(||u+1);
        }
    }

    fn uct(&self,index: usize, player: P, nt: u32) -> (bool,A,f32) {

        match self.stack[index] {
            Node::Terminal(s,a,p,w) => {
                let val = if p == player {w} else {1.0 - w};
                (s,a,val)
            },
            Node::Unknown(s,a) => {
                (s,a,f32::INFINITY)
            },
            Node::Leaf(s,a,p,w,n) |
            Node::Branch(s,a,p,w,n,_) => {
                let nf = n as f64;
                let w = if p == player {w} else {nf - w};
                let avg = self.rave_blend(index,p,player,(w/nf) as f32,nf);
                let n = n as f32;
                let nt = nt as f32;
                let c = self.exploration;
                let val = avg + c*(nt.ln()/n).sqrt();
                (s,a,val)
            },
            Node::Transpose(s,a,u) => {

                //Do not use recursion to allow the compiler to inline
                let v = match self.stack[u] {
                    Node::Terminal(_,_,p,w) => {
                        if p == player {w} else {1.0 - w}
                    },
                    Node::Unknown(_,_) => {
                        f32::INFINITY
                    },
                    Node::Leaf(_,_,p,w,n) |
                    Node::Branch(_,_,p,w,n,_) => {
                        let nf = n as f64;
                        let w = if p == player {w} else {nf - w};
                        let avg = self.rave_blend(u,p,player,(w/nf) as f32,nf);
                        let n = n as f32;
                        let nt = nt as f32;
                        let c = self.exploration;
                        avg + c*(nt.ln()/n).sqrt()
                    },
                    Node::Transpose(_,_,_) => {
                        panic!("should not be possible to transpose to another transpose");
                    }
                };
                (s,a,v)
            }
        }
    }

    fn rollout(&mut self,state: &S) -> f32 {
        let mut sim;
        let mut s = state;
        let p = s.player();
        
        loop {
            if let Some(result) = s.gameover() {
                let side = s.player() == p;
                let v = result.value();
                return if side {v} else {1.0 - v}
            }
            
            self.actions.clear();
            s.actions(&mut |a|{
                self.actions.push(a);
            });

            let max = self.actions.len();
            assert!(
                max > 0,
                "GameState::actions() yielded zero actions for a state with gameover() == None ({:?}). \
                 Every non-terminal state must offer at least one legal action.",
                s
            );

            //use rejection sampling to choose a random action
            let mask = max.next_power_of_two() - 1;
            let chosen;
            loop {
                let r = (self.rand.next_u64() as usize) & mask;
                if r < max {
                    chosen = self.actions[r];
                    sim = s.make(chosen);
                    break;
                }
            }
            if self.use_rave {
                self.played.push(chosen);
            }

            s = &sim;
        }
    }
    
    fn go(&mut self,state: &S, index: usize) -> f32 {
        match self.stack[index] {
            Node::Branch(s,a,player,w,n,c) => {
                let mut selection = None;
                let mut best = -1.0;
                //Reservoir-sample among ties instead of always keeping the first, so selection
                //isn't systematically biased toward whichever action a game's actions() happens
                //to enumerate first (most visibly, every child starts tied at f32::INFINITY
                //before any of them have been tried).
                let mut ties: u64 = 0;
                let mut sibling = Some(c);

                while let Some(u) = sibling {
                    let (s,a,uct) = self.uct(u,player,n);
                    if uct > best {
                        best = uct;
                        selection = Some((a,u));
                        ties = 1;
                    } else if uct == best {
                        ties += 1;
                        if self.rand.next_u64() % ties == 0 {
                            selection = Some((a,u));
                        }
                    }
                    sibling = s.then(||u+1);
                }
                let (action,next_index) = selection.expect("should find a best action");

                let rave_start = self.played.len();
                if self.use_rave {
                    self.played.push(action);
                }

                let next = state.make(action);
                let v = self.go(&next,next_index);

                let v = if next.player() == player {v} else {1.0 - v};
                let w = w + v as f64;
                let n = n + 1;
                self.stack[index] = Node::Branch(s,a,player,w,n,c);

                if self.use_rave {
                    self.rave_update(c,player,v,rave_start);
                }

                //MCTS-Solver: the child just visited may have resolved this branch's own value
                //for certain (see solved_value's doc comment). Once solved, short-circuit to
                //Terminal so future visits here - and, via this same return value, the parent's
                //own solved_value check - see the proven value instead of spending further
                //iterations "reconfirming" statistically what's already known for certain.
                let solved = self.solved_value(c,player);
                let result = match solved {
                    Some(value) => {
                        //`a` here is this node's own "how did my parent reach me" label - load-
                        //bearing for every node except the root, which has no parent to report
                        //it to (its `a` has only ever been an unused placeholder). Repurpose it
                        //at the root specifically to remember the winning/only move, so best()
                        //can still report it once the whole search is solved and this Branch's
                        //child pointer - the only other place that move was recorded - is gone.
                        let remembered_action = if index == 0 {action} else {a};
                        self.stack[index] = Node::Terminal(s,remembered_action,player,value);
                        self.info.branch -= 1;
                        self.info.terminal += 1;
                        value
                    },
                    None => v,
                };

                if index == 0 {
                    self.info.n = n;
                    self.info.q = solved.unwrap_or_else(|| (w/(n as f64)) as f32);
                }

                result
            },
            Node::Leaf(s,a,p,w,n) => {
                if n > self.expansion {
                    let c = self.stack.len();
                    
                    state.actions(&mut |a| {
                        self.stack.push(Node::Unknown(true,a));
                        self.info.unknown += 1;
                        if self.use_rave {
                            self.rave.push((0,0.0));
                        }
                    });

                    assert!(
                        self.stack.len() > c,
                        "GameState::actions() yielded zero actions for a state with gameover() == None ({:?}). \
                         Every non-terminal state must offer at least one legal action.",
                        state
                    );

                    if let Some(Node::Unknown(_,a)) = self.stack.pop() {
                        self.stack.push(Node::Unknown(false,a));
                    }
                    
                    self.stack[index] = Node::Branch(s,a,p,w,n,c);
                    self.info.leaf -= 1;
                    self.info.branch += 1;
                    self.go(state,index)
                } else {
                    let v = if self.use_custom_evaluation {
                        state.custom_evaluation()
                    } else {
                        self.rollout(state)
                    };
                    self.stack[index] = Node::Leaf(s,a,p,w + v as f64,n + 1);
                    v
                }
            },
            Node::Terminal(_,_,_,w) => {
                w
            },
            Node::Unknown(s,a) => {
                
                if self.use_transposition {
                    let h = state.hash();
                    if let Some(&u) = self.map.get(&h) {
                        self.stack[index] = Node::Transpose(s,a,u);
                        self.info.unknown -= 1;
                        self.info.transpose += 1;
                        return self.go(state,u);
                    } else {
                        self.map.insert(h, index);
                    }
                }
                
                let p = state.player();
                if let Some(result) = state.gameover() {   
                    self.stack[index] = Node::Terminal(s,a,p,result.value());
                    self.info.unknown -= 1;
                    self.info.terminal += 1;
                } else {
                    
                    self.stack[index] = Node::Leaf(s,a,p,0.0,0);
                    self.info.unknown -= 1;
                    self.info.leaf += 1;
                }
                
                self.go(state,index)
            },
            Node::Transpose(_,_,u) => {
                self.go(state,u)
            }
        }
    }
}