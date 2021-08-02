mod node;

mod blockchain;

use std::env;
use std::process;

mod encoder;
mod leader_discoverer;
mod leader_down_handler;
mod stdin_reader;
mod utils;

fn port_missing() -> i32 {
    println!("Number of port must be specified");
    return -1;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        process::exit(port_missing());
    }

    let mut node = node::Node::new(&args[1]);
    node.run();
}
