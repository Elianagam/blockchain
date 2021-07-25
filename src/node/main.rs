#[path = "../utils/logger.rs"]
mod logger;

#[path = "../utils/messages.rs"]
mod messages;

mod blockchain;
mod encoder;
mod node;
mod node_leader;
mod node_non_leader;

use std::env;
use std::process;
use std::thread;
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::net::UdpSocket;
use std::io::{self, BufRead};

use blockchain::Blockchain;
use node_leader::run_bully_as_leader;
use node_non_leader::run_bully_as_non_leader;

fn run_bully_thread(bully_sock: UdpSocket, leader_addr: Arc<Mutex<Option<String>>>, stdin_buf: Arc<Mutex<Option<String>>>) -> () {
    let blockchain = Blockchain::new();

    // FIXME: usar condvars
    while leader_addr.lock().unwrap().is_none() {
        std::thread::sleep(Duration::from_secs(1));
    }

    let leader_addr = (*leader_addr.lock().unwrap()).clone().expect("No leader addr");

    let iamleader = leader_addr == bully_sock.local_addr().unwrap().to_string();

    match iamleader {
        true => run_bully_as_leader(bully_sock, blockchain, stdin_buf),
        false => run_bully_as_non_leader(bully_sock, blockchain, leader_addr, stdin_buf),
    }
}

fn usage() -> i32 {
    println!("Usage: cargo r --bin node");
    return -1;
}

fn read_stdin() -> String {
    println!("Ingresar dato a blockchain: ");
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    let line = iterator.next().unwrap().unwrap();
    line
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

    let leader = leader_addr.clone();

    let tmp = stdin_buffer.clone();

    let t = thread::spawn(move || run_bully_thread(bully_sock, leader, tmp));

    thread::spawn(move || {
        let mut node = node::Node::new(bully_sock_addr, leader_addr.clone());
        node.run();
    });

    for _ in 1..10 {
        let t = read_stdin();
        *(&stdin_buffer).lock().unwrap() = Some(t);
    }

    t.join().unwrap();

    Ok(())
}
