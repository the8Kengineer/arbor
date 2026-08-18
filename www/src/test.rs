//Unit tests for the pure, non-DOM logic that drives the visual interface: the move-highlighting
//heatmap (util::colorize) and the two slider-label formatters (components::game_ui). None of
//this touches yew's Component/html! machinery or web-sys, so it runs as plain native `cargo
//test` - no wasm target or browser needed.
use crate::util::colorize;
use crate::components::game_ui::{fmt_ai_time,fmt_ai_eve};

#[test]
fn colorize_ranks_the_extremes_and_the_average_into_distinct_bands() {
    let weighted = vec![("max",1.0f32), ("mid",0.5), ("min",0.0)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    assert_eq!(classed,vec![("max","pos-75p"), ("mid","pos-0p"), ("min","neg-75p")]);
}

#[test]
fn colorize_preserves_the_order_and_identity_of_the_input() {
    let weighted = vec![(3,0.9f32), (1,0.1), (2,0.5)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    let ids: Vec<i32> = classed.iter().map(|(a,_)| *a).collect();
    assert_eq!(ids,vec![3,1,2]);
}

#[test]
fn colorize_clears_stale_entries_from_a_previous_call() {
    //classed is reused across renders (see GameUI::ponder) rather than freshly allocated each
    //time, so a caller that forgot to clear it first would otherwise see highlighting from the
    //previous board position bleed into the new one.
    let mut classed = vec![(999,"leftover-from-a-previous-position")];
    let weighted = vec![(1,0.5f32)];
    colorize(&weighted,&mut classed);

    assert_eq!(classed.len(),1);
    assert_ne!(classed[0].0,999);
}

#[test]
fn colorize_does_not_panic_on_an_empty_input() {
    let weighted: Vec<(i32,f32)> = Vec::new();
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);
    assert!(classed.is_empty());
}

//Characterization test, not an endorsement: when every action has exactly the same estimated
//value (a real, common case - e.g. right at the start of a search before any stats differ),
//`scale` divides out to 0.0 and every normalized value becomes 0.0/0.0 = NaN. Every `f < ...`
//comparison against NaN is false, so execution falls through to the final `else` and every
//single action gets painted "pos-75p" (strongest green) - visually claiming the AI has a clear
//favorite when in fact none of the options are distinguishable yet. This pins down the current
//behavior so a future change to the scaling logic is a deliberate choice, not a surprise.
#[test]
fn colorize_marks_every_action_as_strongly_favored_when_all_weights_are_tied() {
    let weighted = vec![(1,0.5f32), (2,0.5), (3,0.5)];
    let mut classed = Vec::new();
    colorize(&weighted,&mut classed);

    assert!(classed.iter().all(|(_,c)| *c == "pos-75p"),
        "expected the current (NaN-driven) all-green fallback, got {:?}",classed);
}

#[test]
fn fmt_ai_time_displays_the_raw_second_count() {
    assert_eq!(fmt_ai_time(1),"1");
    assert_eq!(fmt_ai_time(20),"20");
}

#[test]
fn fmt_ai_eve_scales_the_raw_slider_value_down_to_the_real_exploration_constant() {
    //The "Exploration" slider ranges 20..=40 (see GameUI::view's <Setting> for ai-eve);
    //fmt_ai_eve turns that raw tick count into the actual exploration constant that gets passed
    //to with_exploration (see GameUI::ponder, which divides by 20.0 the same way) - if these two
    //divisors ever drift apart, the displayed number would stop matching what the search is
    //actually using.
    assert_eq!(fmt_ai_eve(20),"1.00");
    assert_eq!(fmt_ai_eve(28),"1.40");
    assert_eq!(fmt_ai_eve(40),"2.00");
}
