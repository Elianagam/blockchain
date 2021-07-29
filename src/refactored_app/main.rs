mod node;

#[path = "blockchain/blockchain.rs"]
mod blockchain;
use blockchain::Blockchain;

#[path = "blockchain/block.rs"]
mod block;
use block::Block;

#[path = "blockchain/student.rs"]
mod student;
use student::Student;

#[path = "blockchain/record.rs"]
mod record;
use record::Record;

use std::env;
use std::process;

mod encoder;

#[path = "../utils/messages.rs"]
mod messages;

fn port_missing() -> i32 {
    println!("Number of port must be specified");
    return -1;
}

fn main() 
{
    let args: Vec<String> = env::args().collect();
    if args.len() < 2
    {
        process::exit(port_missing());
    }
    let mut node = node::Node::new(&args[1]);
    node.run();
}