use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub label: Option<(bool,u8)>,
    pub make: Callback<()>,
    pub color: &'static str,
}

#[function_component(Square)]
pub fn square(props: &Props) -> Html {
    let Props {label, make, color} = props.clone();
    let onclick = Callback::from(move |_e| make.emit(()));

    let (side_class, text) = match label {
        Some((true,n)) => ("side-a", n.to_string()),
        Some((false,n)) => ("side-b", n.to_string()),
        None => ("empty", String::new()),
    };

    html! {
        <div class={format!("onestone-cell {} {}",side_class,color)} {onclick}>
            {text}
        </div>
    }
}
