use yew::prelude::*;
use crate::components::game_ui::*;
use super::board::Board;
use checkers::*;
use arbor::*;

impl GIPlayer for Side {}
impl GIAction for Move {}

impl GameInstance<Side,Move> for Checkers {
    fn new() -> Self {
        Checkers::new()
    }

    fn name() -> &'static str {
        "Checkers"
    }

    fn status(&self) -> String {
        let side = self.player();
        let other = match side {
            Side::Red => Side::White,
            Side::White => Side::Red,
        };
        if let Some(result) = self.gameover() {
            match result {
                GameResult::Draw => format!("Draw!"),
                GameResult::Win  => format!("{} wins!", side),
                GameResult::Lose => format!("{} wins!", other),
            }
        } else {
            format!("{} to play", side)
        }
    }

    fn view(&self, make: yew::Callback<Move>, actions: Vec<(Move,&'static str)>) -> Html {
        html! {
            <Board {actions} square={self.square} {make}/>
        }
    }
}
