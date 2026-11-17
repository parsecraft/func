//! Counts words in a file, or in stdin. Uses a crate. Try: func run examples/wordcount.rs README.md
// dep: regex = "1"
// edition: 2021

use regex::Regex;
use std::io::Read;

fn main() {
    let text = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|error| {
            eprintln!("cannot read {path}: {error}");
            std::process::exit(1);
        }),
        None => {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text).expect("stdin is not valid text");
            text
        }
    };
    let words = Regex::new(r"\w+").expect("valid pattern").find_iter(&text).count();
    println!("{words}");
}
