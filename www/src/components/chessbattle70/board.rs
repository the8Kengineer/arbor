use yew::prelude::*;
use chessbattle70::{Board, Move, Pos, HEIGHT, WIDTH};
use super::square::Square;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub actions: Vec<(Move, &'static str)>,
    pub board: Board,
    pub make: Callback<Move>,
}

//Unlike every other game in this app, a click can't map directly to one action here - the same
//square can be one piece's destination and another piece's source, and most squares aren't
//reachable at all from the current selection. So play is two clicks: click one of your own
//pieces that has a legal move (highlighted) to select it, then click one of its highlighted
//destinations to play it (or click the selected piece again to deselect).
#[function_component(BoardComponent)]
pub fn board(props: &Props) -> Html {
    let Props { actions, board, make } = props;
    let board = *board;

    let selected = use_state(|| Option::<Pos>::None);

    let is_source = |pos: Pos| actions.iter().any(|(m, _)| m.from == pos);

    let mut squares = Vec::new();
    for row in 0..HEIGHT {
        for col in 0..WIDTH {
            let pos = Pos::new(col, row);
            let piece = board.get(pos);

            // "neutral" (an actual action, just not yet AI-scored) needs to look different
            // from a square that isn't clickable at all - the latter stays "inert".
            let mut color = "inert";
            let mut onclick = Callback::from(move |()| ());

            if let Some(from) = *selected {
                if pos == from {
                    color = "select-source";
                    let selected = selected.clone();
                    onclick = Callback::from(move |()| selected.set(None));
                } else if let Some((mv, c)) = actions.iter().find(|(m, _)| m.from == from && m.to == pos) {
                    color = *c;
                    let make = make.clone();
                    let mv = *mv;
                    let selected = selected.clone();
                    onclick = Callback::from(move |()| {
                        selected.set(None);
                        make.emit(mv);
                    });
                } else if is_source(pos) {
                    let selected = selected.clone();
                    onclick = Callback::from(move |()| selected.set(Some(pos)));
                }
            } else if is_source(pos) {
                color = "select-source";
                let selected = selected.clone();
                onclick = Callback::from(move |()| selected.set(Some(pos)));
            }

            let light = (row + col) % 2 == 0;

            squares.push(html! {
                <Square {piece} {light} {onclick} {color}/>
            });
        }
    }

    html! {
        <div class="board-container-parent chessbattle70-container">
            <div class="board-container-child chessbattle70-board">
                {squares}
            </div>
        </div>
    }
}
