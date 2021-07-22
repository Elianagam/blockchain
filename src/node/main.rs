use std::env;
use std::process;

use std::thread;

mod blockchain;
mod encoder;
mod node;

mod node_leader;
use node_leader::*;
mod node_non_leader;
use node_non_leader::*;

use blockchain::Blockchain;

fn run_bully_thread(iamleader: bool) -> () {
    let blockchain = Blockchain::new();

    match iamleader {
        true => run_bully_as_leader(blockchain),
        false => run_bully_as_non_leader(blockchain),
    }
}

fn usage() -> i32 {
    println!("Usage: cargo r --bin node [--leader]");
    return -1;
}

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        process::exit(usage());
    }

    let iamleader: bool = args.len() > 1 && args[1] == "--leader";
    let mut node = node::Node::new();
    let t = thread::spawn(move || run_bully_thread(iamleader));

    node.run();

    t.join().unwrap();

    Ok(())
}
