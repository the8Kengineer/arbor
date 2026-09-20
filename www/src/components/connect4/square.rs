use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub red: bool,
    pub yellow: bool,
    pub make: Callback<()>,
    pub color: &'static str,
}

#[function_component(Square)]
pub fn square(props: &Props) -> Html {
    let Props {red, yellow, make, color} = props.clone();

    let onclick = Callback::from(move |_e| make.emit(()));
    let color =
        if red {
            "red"
        } else if yellow {
            "yellow"
        } else {
            color
        };
    let class = format!("connect4-square {}", color);
    html! {
        <div
            {class}
            {onclick}>
        </div>
    }
}