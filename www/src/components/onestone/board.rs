use yew::prelude::*;
use onestone::{Move,Side,Square as Cell};
use super::square::Square;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub actions: Vec<(Move,&'static str)>,
    pub board: [Cell;25],
    pub side: Side,
    pub make: Callback<Move>,
}

enum Click {
    None,
    Move(Move),
    Select(u8),
    Deselect,
}

//A die roll can leave a genuine choice between two different cubes (the nearest lower and
//nearest higher numbered cubes still on the board, when the exact rolled number has already
//been captured). Since a Move only pins down a destination square, not which cube is moving
//there, two different candidate cubes can legally land on the very same square. Clicking
//squares alone can't disambiguate that, so this board asks the player to pick a source cube
//first whenever more than one candidate exists, then shows just that cube's destinations.
//When there's only one candidate (the common case), it's auto-picked and play stays a single
//click, matching the other games in this app.
#[function_component(Board)]
pub fn board(props: &Props) -> Html {
    let Props {actions, board, side, make} = props;
    let side = *side;
    let board = *board;

    let selected = use_state(|| Option::<u8>::None);

    if actions.len() == 1 && actions[0].0.number == 0 {
        let make = make.clone();
        let onclick = Callback::from(move |_: MouseEvent| make.emit(Move {number: 0, to: usize::MAX}));
        return html! {
            <div class="board-container-parent">
                <div class="board-container-child onestone-board onestone-blocked">
                    <div class="onestone-pass" {onclick}>{"No legal moves - pass"}</div>
                </div>
            </div>
        };
    }

    let mut candidates: Vec<u8> = Vec::new();
    for (a,_) in actions.iter() {
        if !candidates.contains(&a.number) {
            candidates.push(a.number);
        }
    }

    let picked: Option<u8> = if candidates.len() <= 1 {
        candidates.first().copied()
    } else {
        selected.filter(|n| candidates.contains(n))
    };

    let position_of = |number: u8| -> Option<usize> {
        (0..25).find(|&i| matches!(board[i], Cell::Piece(s,n) if s == side && n == number))
    };

    let mut squares = Vec::new();
    for i in 0..25 {
        let label = match board[i] {
            Cell::Empty => None,
            Cell::Piece(Side::A,n) => Some((true,n)),
            Cell::Piece(Side::B,n) => Some((false,n)),
        };

        let mut color = "neutral";
        let mut click = Click::None;

        if let Some(number) = picked {
            for (a,c) in actions.iter() {
                if a.number == number && a.to == i {
                    color = *c;
                    click = Click::Move(*a);
                }
            }

            if matches!(click, Click::None) && candidates.len() == 2 && position_of(number) == Some(i) {
                color = "select-source";
                click = Click::Deselect;
            }
        } else {
            for &number in candidates.iter() {
                if position_of(number) == Some(i) {
                    color = "select-source";
                    click = Click::Select(number);
                }
            }
        }

        let make = make.clone();
        let selected = selected.clone();
        let onclick = match click {
            Click::Move(mv) => Callback::from(move |()| {
                selected.set(None);
                make.emit(mv);
            }),
            Click::Select(number) => Callback::from(move |()| selected.set(Some(number))),
            Click::Deselect => Callback::from(move |()| selected.set(None)),
            Click::None => Callback::from(move |()| ()),
        };

        squares.push(html! {
            <Square {label} make={onclick} {color}/>
        });
    }

    html! {
        <div class="board-container-parent">
            <div class="board-container-child onestone-board">
                {squares}
            </div>
        </div>
    }
}
