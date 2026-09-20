use yew::prelude::*;
use crate::components::game_ui::*;
use super::board::Board;
use onestone::*;
use arbor::*;

impl GIPlayer for Side {}
impl GIAction for Move {}

//OneStone ends one of two ways - `winner` reaches their own home corner, or `winner`'s opponent
//has no pieces left. Mirrors the check inside Onestone::gameover() (onestone/src/onestone.rs)
//using only its public board/Side/Square types, so the web UI can name which one actually
//happened rather than a single generic "wins" message.
fn win_reason(game: &Onestone, winner: Side) -> &'static str {
    let target_corner = match winner {
        Side::A => 24,
        Side::B => 0,
    };
    let reached = matches!(game.board[target_corner], Square::Piece(s,_) if s == winner);
    if reached { "Corner Reached!" } else { "All Captured!" }
}

impl GameInstance<Side,Move> for Onestone {
    fn new() -> Self {
        Onestone::new()
    }

    fn name() -> &'static str {
        "OneStone"
    }

    fn status(&self) -> String {
        let side = self.player();
        let other = match side {
            Side::A => Side::B,
            Side::B => Side::A,
        };
        if let Some(result) = self.gameover() {
            match result {
                GameResult::Draw => format!("Draw!"),
                GameResult::Win  => format!("{} - Side {:?} Wins", win_reason(self,side), side),
                GameResult::Lose => format!("{} - Side {:?} Wins", win_reason(self,other), other),
            }
        } else {
            format!("Side {:?} to play, die: {}", side, self.die)
        }
    }

    fn view(&self, make: yew::Callback<Move>, actions: Vec<(Move,&'static str)>) -> Html {
        html! {
            <Board {actions} board={self.board} side={self.side} {make}/>
        }
    }
}
