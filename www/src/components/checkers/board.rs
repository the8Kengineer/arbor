use yew::prelude::*;
use checkers::{Move,Square as Cell,N};
use super::square::Square;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub actions: Vec<(Move,&'static str)>,
    pub square: [Cell;N],
    pub make: Callback<Move>,
}

enum Click {
    None,
    Move(Move),
    Select(u8),
    Deselect,
}

// Standard checkers numbering: only the 32 dark squares are playable. This
// mirrors the private geometry in the checkers crate (row*4 + col/2),
// duplicated here since the game only exposes the flat 32-square array, not
// the row/col conversion helpers.
fn square_at(row: i32, col: i32) -> Option<u8> {
    if !(0..8).contains(&row) || !(0..8).contains(&col) {
        return None;
    }
    if (row + col) % 2 == 0 {
        return None;
    }
    Some((row*4 + col/2) as u8)
}

// A move is identified by a (from,to) pair rather than a single destination,
// so unlike most of the other boards in this app, a click on the piece to
// move is needed before the destination squares light up. When only one
// piece has a legal move (the common case, especially mid multi-jump), it's
// auto-picked and play stays a single click.
#[function_component(Board)]
pub fn board(props: &Props) -> Html {
    let Props {actions, square, make} = props;
    let square = *square;

    let selected = use_state(|| Option::<u8>::None);

    let mut froms: Vec<u8> = Vec::new();
    for (a,_) in actions.iter() {
        if !froms.contains(&a.from) {
            froms.push(a.from);
        }
    }

    let picked: Option<u8> = if froms.len() <= 1 {
        froms.first().copied()
    } else {
        selected.filter(|f| froms.contains(f))
    };

    let mut cells = Vec::new();
    for row in (0..8).rev() {
        for col in 0..8 {
            let cell = match square_at(row,col) {
                None => html! {<div class="checkers-cell checkers-light"/>},
                Some(sq) => {
                    let piece = match square[sq as usize] {
                        Cell::Empty => None,
                        Cell::Occupied(s,k) => Some((s,k)),
                    };

                    let mut color = "neutral";
                    let mut click = Click::None;

                    if let Some(from) = picked {
                        for (a,c) in actions.iter() {
                            if a.from == from && a.to == sq {
                                color = *c;
                                click = Click::Move(*a);
                            }
                        }

                        if matches!(click, Click::None) && froms.len() > 1 {
                            if sq == from {
                                color = "select-source";
                                click = Click::Deselect;
                            } else if froms.contains(&sq) {
                                color = "select-source";
                                click = Click::Select(sq);
                            }
                        }
                    } else if froms.contains(&sq) {
                        color = "select-source";
                        click = Click::Select(sq);
                    }

                    let make = make.clone();
                    let selected = selected.clone();
                    let onclick = match click {
                        Click::Move(mv) => Callback::from(move |()| {
                            selected.set(None);
                            make.emit(mv);
                        }),
                        Click::Select(f) => Callback::from(move |()| selected.set(Some(f))),
                        Click::Deselect => Callback::from(move |()| selected.set(None)),
                        Click::None => Callback::from(move |()| ()),
                    };

                    html! {<Square {piece} make={onclick} {color}/>}
                }
            };
            cells.push(cell);
        }
    }

    html! {
        <div class="board-container-parent">
            <div class="board-container-child checkers-board">
                {cells}
            </div>
        </div>
    }
}
