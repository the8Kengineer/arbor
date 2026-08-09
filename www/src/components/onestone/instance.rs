use yew::prelude::*;
use crate::components::game_ui::*;
use super::board::Board;
use onestone::*;
use arbor::*;

impl GIPlayer for Side {}
impl GIAction for Move {}

impl GameInstance<Side,Move> for Onestone {
    fn new() -> Self {
        Onestone::new()
    }

    fn name() -> &'static str {
        "EinStein würfelt nicht!"
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
                GameResult::Win  => format!("Side {:?} wins!", side),
                GameResult::Lose => format!("Side {:?} wins!", other),
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
