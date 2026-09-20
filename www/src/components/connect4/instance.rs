use yew::prelude::*;
use crate::components::game_ui::*;
use super::board::Board;
use connect4::connect4::*;
use arbor::*;

impl GIPlayer for Disc {}
impl GIAction for Column {}

fn fmt_disc(disc: &Disc) -> &'static str {
    match disc {
        Disc::R => "Blue",
        Disc::Y => "Yellow",
        Disc::N => "Neither",
    }
}

impl GameInstance<Disc,Column> for Connect4 {
    fn new() -> Self {
        Connect4::new()
    }

    fn name() -> &'static str {
        "Connect 4"
    }

    fn status(&self) -> String {
        let side = self.player();
        let other = match side {
            Disc::Y => Disc::R,
            Disc::R => Disc::Y,
            Disc::N => Disc::N,
        };
        if let Some(result) = self.gameover() {
            match result {
                GameResult::Draw => format!("Board Full! - Draw"),
                GameResult::Win  => format!("4 in a Row! - {} Wins", fmt_disc(&side)),
                GameResult::Lose  => format!("4 in a Row! - {} Wins", fmt_disc(&other)),
            }
        } else {
            format!("{}'s turn", fmt_disc(&side))
        }
    }
    
    fn view(&self, make: yew::Callback<Column>, actions: Vec<(Column,&'static str)>) -> Html {
        let squares = self.space;
        html! {
            <Board {actions} {squares} {make}/>
        }
    }

}