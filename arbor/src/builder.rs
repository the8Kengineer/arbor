use super::*;


impl<P: Player, A: Action, S: GameState<P,A>> MCTS<P, A, S> {
    
    ///Sets the exploration parameter. This sets the balance between exploration and exploitation when MCTS determines which action to choose. Set to a value > 0.
    pub fn with_exploration(mut self, exploration: f32) -> Self {
        assert!(exploration > 0.0,"A positive value is required for the exploration constant.");
        self.exploration = exploration;
        self
    }

    ///Sets the expansion parameter. This is the minimum number of times a leaf node should be visited before expanding it into a branch node.
    pub fn with_expansion_minimum(mut self, expansion: u32) -> Self {
        assert!(expansion > 0,"The value for expansion minimum must be greater than zero.");
        self.expansion = expansion;
        self
    }

    ///Enables the custom evaluation method.
    pub fn with_custom_evaluation(mut self) -> Self {
        self.use_custom_evaluation = true;
        self
    }
    
    ///Enables transposition detection. Experimental.
    pub fn with_transposition(mut self) -> Self {
        self.use_transposition = true;
        self
    }

    ///Enables RAVE (Rapid Action Value Estimation), a form of AMAF (all-moves-as-first) that shares statistics for a move across every simulation that plays it anywhere later in the tree, not just the simulations that visited it as the immediate next move from a given position. This gives much faster early convergence than plain UCT, at the cost of a bias that fades out (via the standard beta = sqrt(k / (3n + k)) blend) as a node accumulates real visits. Off by default since it changes search behavior; opt in explicitly.
    pub fn with_rave(mut self) -> Self {
        self.use_rave = true;
        self
    }

    ///Seeds the internal random number generator from entropy. This is inteneded to produce non-deterministic search results.
    pub fn with_entropy(mut self) -> Self {
        use rand::SeedableRng;
        self.rand = Rng::from_entropy();
        self
    }
}