extern crate arbor;

mod checkers;
use std::io;
use std::io::prelude::*;
use self::checkers::*;
use arbor::*;
use instant::Instant;

fn main() {
    println!("Checkers!");

    let mut gamestate = Checkers::new();
    println!("{}",gamestate);

    loop {
        if let Some(result) = gamestate.gameover() {
            match result {
                GameResult::Draw => println!("Draw!"),
                GameResult::Win  => println!("{} wins!",gamestate.side),
                GameResult::Lose => println!("{} loses!",gamestate.side),
            }
            break;
        }

        if gamestate.side == Side::Red {
            let mut moves = Vec::new();
            gamestate.actions(&mut |m| moves.push(m));

            for (i,m) in moves.iter().enumerate() {
                println!("[{}] {} -> {}",i,m.from,m.to);
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
            let mut mcts = MCTS::new(gamestate);
            let duration = std::time::Duration::new(1, 0);
            let start = Instant::now();

            while (Instant::now() - start) < duration {
                mcts.ponder(100);
            }

            let action = mcts.best().expect("Should find a best action");

            println!("{:?}",mcts.info);
            println!("{} -> {}",action.from,action.to);
            gamestate = gamestate.make(action);
        }

        println!("{}",gamestate);
    }
}

#[cfg(test)]
mod test;
