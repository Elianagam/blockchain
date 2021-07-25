#[path = "../utils/logger.rs"]
mod logger;

#[path = "../utils/messages.rs"]
mod messages;

mod blockchain;
mod encoder;
mod node;

use std::env;
use std::io::{self, BufRead};
use std::net::UdpSocket;
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
    let bully_sock = UdpSocket::bind("0.0.0.0:0").unwrap();
    let bully_sock_addr = bully_sock.local_addr().unwrap().to_string();
    let leader_addr = Arc::new(Mutex::new(None));
    let tmp = stdin_buffer.clone();
    //let t = thread::spawn(move || run_bully_thread(bully_sock, leader, tmp));

    let t = thread::spawn(move || {
        let mut node = node::Node::new(bully_sock_addr, leader_addr.clone());
        node.run(bully_sock, leader_addr.clone(), tmp);
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
