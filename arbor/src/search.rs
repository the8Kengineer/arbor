use super::*;
use rand::SeedableRng;
use rand::RngCore;

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
            info: Info::default(),
            root: root,
            stack: Vec::new(),
            actions: Vec::new(),
            rand: Rng::from_seed(s),
            map: HashMap::default(),
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
            
            self.info.leaf = 1;
            
            //Call go once with expansion set to zero to force the root to expand 
            let root = self.root;
            let expansion = self.expansion;
            self.expansion = 0;
            self.go(&root, 0);
            self.expansion = expansion;
            self.ponder(n - 1);
        } else {
            let root = self.root;
            for _ in 0..n {
                self.go(&root,0);
            }
            
            self.info.bytes = self.stack.len() * std::mem::size_of::<Node<P,A>>();
        }
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
                let avg = (w/nf) as f32;
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
                        let avg = (w/nf) as f32;
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
            loop {
                let r = (self.rand.next_u64() as usize) & mask;
                if r < max {
                    sim = s.make(self.actions[r]);
                    break;
                }
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
                let next = state.make(action);
                let v = self.go(&next,next_index);

                let v = if next.player() == player {v} else {1.0 - v};
                let w = w + v as f64;
                let n = n + 1;
                self.stack[index] = Node::Branch(s,a,player,w,n,c);

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