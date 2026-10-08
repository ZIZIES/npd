mod cmd;
mod store;

use std::{env, process};

fn main() {
    // skip(1) drops argv[0], the program name
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(msg) = cmd::run(&args) {
        eprintln!("npd: {msg}");
        process::exit(1);
    }
}
