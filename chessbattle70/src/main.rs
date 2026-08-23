extern crate arbor;

mod board;
mod check;
mod game;
mod movegen;
mod moves;
mod piece;
mod rules;
mod setup;

use std::io;
use std::io::prelude::*;
use self::board::*;
use self::game::*;
use self::moves::*;
use self::piece::*;
use self::rules::*;
use arbor::{GameResult, GameState, MCTS};
use instant::Instant;

fn main() {
    println!("\nChessBattle70\n");

    let mut gamestate = Game::new();
    println!("{}", gamestate);

    loop {
        if let Some(result) = gamestate.gameover() {
            match result {
                GameResult::Draw => println!("Draw!"),
                GameResult::Win  => println!("{:?} wins!", gamestate.current_player),
                GameResult::Lose => println!("{:?} loses - checkmate!", gamestate.current_player),
            }
            break;
        }

        if gamestate.current_player == Player::White {
            let mut moves = Vec::new();
            gamestate.actions(&mut |m| moves.push(m));

            for (i, m) in moves.iter().enumerate() {
                println!("[{}] {:?} ({},{}) -> ({},{})", i, m.piece.kind, m.from.col, m.from.row, m.to.col, m.to.row);
            }

            print!("=> ");
            //flushes standard out so the print statements are actually displayed
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if let Err(_) = io::stdin().read_line(&mut input) {
                println!("Failed to read user input");
                continue;
            }

            if let Ok(i) = input.trim().parse::<usize>() {
                if i < moves.len() {
                    gamestate = gamestate.make(moves[i]);
                } else {
                    println!("invalid selection");
                    continue;
                }
            } else {
                println!("parse failed");
                continue;
            }
        } else {
            let mut mcts = MCTS::new(gamestate).with_custom_evaluation();
            let duration = std::time::Duration::new(1, 0);
            let start = Instant::now();

            while (Instant::now() - start) < duration {
                mcts.ponder(100);
            }

            let action = mcts.best().expect("Should find a best action");

            println!("{:?}", mcts.info);
            println!("{:?} ({},{}) -> ({},{})", action.piece.kind, action.from.col, action.from.row, action.to.col, action.to.row);
            gamestate = gamestate.make(action);
        }

        println!("{}", gamestate);
    }
}

#[cfg(test)]
mod test;
