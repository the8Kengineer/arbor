//! Which movement rules govern piece slots #4 and #5.
//!
//! The physical piece (and its graphic) is the same in all three variants;
//! only how it moves changes. Falcon==Hawk and Mammoth==Elephant are the
//! same slot under different names.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaptorPachydermRules {
    /// Default. Falcon: forced exactly-3-orthogonal-unit-step move, free
    /// turns between steps, first two steps airborne (fly over anything),
    /// only the final square can capture -- 16 reachable squares total.
    /// Mammoth: moves like a rook, but on capturing may choose to trample
    /// through the captured square and keep sliding rather than stopping.
    FalconMammoth,
    /// Seirawan-Harper. Hawk = Knight + Bishop moves combined (both leap
    /// and slide normally, blocked as usual). Elephant = Knight + Rook
    /// moves combined, same normal blocking rules.
    SeirawanHarper,
    /// "Realism" variant: the Elephant's knight-shaped move requires an
    /// unobstructed orthogonal L-path to reach its destination (checked
    /// via both possible L-routes; either being clear is enough). The
    /// Hawk's knight- and bishop-shaped moves both ignore blocking pieces
    /// entirely ("it flies") -- only the landing square's occupant matters.
    Realism,
}

impl Default for RaptorPachydermRules {
    fn default() -> Self {
        RaptorPachydermRules::FalconMammoth
    }
}
