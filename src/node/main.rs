#[path = "../utils/logger.rs"]
mod logger;

#[path = "../utils/messages.rs"]
mod messages;

#[path = "blockchain/block.rs"]
mod block;
use block::Block;

#[path = "blockchain/blockchain.rs"]
mod blockchain;
use blockchain::Blockchain;

#[path = "blockchain/student.rs"]
mod student;
use student::Student;

#[path = "blockchain/record.rs"]
mod record;
use record::Record;

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

        let student_data: Vec<&str> = line.split(",").collect();
        if (student_data.len() == 1 && student_data[0] == "close") || student_data.len() == 2 {
            *(&stdin_buffer).lock().unwrap() = Some(line);
        } else {
            println!("Unsupported data format, usage: id, qualification")
        }
    }

    t.join().unwrap();

    Ok(())
}
