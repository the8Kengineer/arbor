use yew::prelude::*;
use crate::components::game_ui::*;
use super::board::BoardComponent;
use chessbattle70::*;
use chessbattle70::check::{game_status, GameStatus};

impl GIPlayer for Player {}
impl GIAction for Move {}

impl GameInstance<Player, Move> for Game {
    fn new() -> Self {
        Game::new()
    }

    fn name() -> &'static str {
        "ChessBattle70"
    }

    fn use_custom_evaluation() -> bool {
        true
    }

    fn status(&self) -> String {
        match game_status(&self.board, self.current_player, self.rules) {
            GameStatus::InPlay => format!("{:?} to move", self.current_player),
            GameStatus::Checkmate(loser) => {
                let winner = match loser {
                    Player::White => Player::Black,
                    Player::Black => Player::White,
                };
                format!("Checkmate! {:?} wins", winner)
            }
            GameStatus::Stalemate(_) => "Stalemate - draw!".to_string(),
        }
    }

    fn view(&self, make: yew::Callback<Move>, actions: Vec<(Move, &'static str)>) -> Html {
        html! {
            <BoardComponent {actions} board={self.board} {make}/>
        }
    }
}
