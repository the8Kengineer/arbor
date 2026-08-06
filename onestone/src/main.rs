extern crate arbor;

mod onestone;
//use std::io;
//use std::io::prelude::*;
//use self::onsestone::*;
//use arbor::*;
//use instant::Instant;

fn main() {
    println!("\nWhen your only tool is a hammer....\n");

    println!("|-----|-----|-----|-----|-----|");
    println!("| [1] | [2] | [4] |     |     |");
    println!("|-----|-----|-----|-----|-----|");
    println!("| [3] | [5] |     |     |     |");
    println!("|-----|-----|-----|-----|-----|");
    println!("| [6] |     |     |     | (6) |");
    println!("|-----|-----|-----|-----|-----|");
    println!("|     |     |     | (5) | (3) |");
    println!("|-----|-----|-----|-----|-----|");
    println!("|     |     | (4) | (2) | (1) |");
    println!("|-----|-----|-----|-----|-----|");

    println!("\nEinStein würfelt nicht!\n");
}

#[cfg(test)]
mod test;