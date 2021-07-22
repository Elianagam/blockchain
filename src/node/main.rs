use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::process;
use std::str;
use std::thread;

mod blockchain;
mod encoder;
mod node;
use encoder::*;

use blockchain::{Block, Blockchain};

const LEADER_ADDR: &str = "127.0.0.1:8000";
const DUMMY_MSG: &str = "testing";
const REGISTER_MSG: &str = "register";

fn run_bully_as_non_leader(mut blockchain: Blockchain) {
    // Let the OS to pick one addr + port for us
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("Enviando mensaje {} al lider", DUMMY_MSG);
    socket
        .send_to(&encode_to_bytes(REGISTER_MSG), LEADER_ADDR)
        .unwrap();

    socket
        .send_to(&encode_to_bytes(DUMMY_MSG), LEADER_ADDR)
        .unwrap();

    for _ in 0..3 {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();

        let msg = decode_from_bytes(buf.to_vec());
        println!("Recibido {}", &msg);

        blockchain.add(Block { data: msg });
    }

    println!("Blockchain final: {:?}", blockchain);
}

fn run_bully_as_leader(mut blockchain: Blockchain) {
    let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

    println!("Soy el líder!");
    let mut other_nodes: Vec<SocketAddr> = vec![];

    let mut propagated_msgs = 0;

    loop {
        let mut buf = [0; 128];
        let (_, from) = socket.recv_from(&mut buf).unwrap();

        let msg = decode_from_bytes(buf.to_vec());

        if propagated_msgs == 3 {
            break;
        }

        match msg.as_str() {
            REGISTER_MSG => {
                println!("Registrando nodo: {}", from);
                if !&other_nodes.contains(&from) {
                    other_nodes.push(from);
                }
            }
            msg => {
                println!("Propagando cambios {:?} al resto de los nodos", msg);
                for node in &other_nodes {
                    socket.send_to(&encode_to_bytes(msg), node).unwrap();
                }
                blockchain.add(Block {
                    data: msg.to_string(),
                });
                propagated_msgs += 1;
            }
        }
    }
}

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
