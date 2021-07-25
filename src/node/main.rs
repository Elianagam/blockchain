#[path = "../utils/logger.rs"]
mod logger;

#[path = "../utils/messages.rs"]
mod messages;

mod blockchain;
mod encoder;
mod node;

use std::env;
use std::io::{self, BufRead};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;

fn usage() -> i32 {
    println!("Usage: cargo r --bin node");
    return -1;
}

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        process::exit(usage());
    }
    let stdin_buffer = Arc::new(Mutex::new(None));
    let tmp = stdin_buffer.clone();

    let t = thread::spawn(move || {
        let mut node = node::Node::new();
        node.run(tmp);
    });

    for _ in 1..10 {
        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        let line = iterator.next().unwrap().unwrap();
        *(&stdin_buffer).lock().unwrap() = Some(line);
    }

    t.join().unwrap();

    Ok(())
}
