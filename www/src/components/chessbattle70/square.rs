use yew::prelude::*;
use chessbattle70::{Piece, PieceType, Player};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub piece: Option<Piece>,
    pub light: bool,
    pub color: &'static str,
    pub onclick: Callback<()>,
}

fn graphic_path(piece: Piece) -> String {
    let side = if piece.player == Player::White { "w" } else { "b" };
    // Falcon/Mammoth are physically the same pieces as the Seirawan-Harper
    // Hawk/Elephant (see rules::RaptorPachydermRules) - hawk/elephant are
    // the only graphics provided, used regardless of the active ruleset.
    let name = match piece.kind {
        PieceType::Pawn => "pawn",
        PieceType::Knight => "knight",
        PieceType::Bishop => "bishop",
        PieceType::Falcon => "hawk",
        PieceType::Mammoth => "elephant",
        PieceType::Rook => "rook",
        PieceType::Queen => "queen",
        PieceType::King => "king",
    };
    format!("graphics/{}{}.gif", side, name)
}

#[function_component(Square)]
pub fn square(props: &Props) -> Html {
    let Props { piece, light, color, onclick } = props.clone();
    let onclick = Callback::from(move |_e: MouseEvent| onclick.emit(()));
    let shade = if light { "light" } else { "dark" };

    html! {
        <div class={format!("chessbattle70-square {} {}", shade, color)} {onclick}>
            if let Some(piece) = piece {
                <img class="chessbattle70-piece" src={graphic_path(piece)} alt="" />
            }
        </div>
    }
}
