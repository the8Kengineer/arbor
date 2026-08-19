use yew::prelude::*;
use checkers::{Side,Kind};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub piece: Option<(Side,Kind)>,
    pub make: Callback<()>,
    pub color: &'static str,
}

#[function_component(Square)]
pub fn square(props: &Props) -> Html {
    let Props {piece, make, color} = props.clone();
    let onclick = Callback::from(move |_e| make.emit(()));

    let side_class = match piece {
        Some((Side::Red,_)) => "side-red",
        Some((Side::White,_)) => "side-white",
        None => "empty",
    };

    let kind_class = match piece {
        Some((_,Kind::King)) => "king",
        Some((_,Kind::Man)) => "man",
        None => "",
    };

    html! {
        <div class={format!("checkers-cell {} {} {}",side_class,kind_class,color)} {onclick}>
            <div class="checkers-piece"></div>
        </div>
    }
}
