use crate::encoder::Encoder;

use std::str;

use crate::blockchain::{Block, Blockchain};
use std::net::UdpSocket;

const LEADER_ADDR: &str = "127.0.0.1:8000";
const DUMMY_MSG: &str = "testing";
const REGISTER_MSG: &str = "register";
const NEW_NODE: &str = "new_node";
const END: &str = "-";

fn recv_all_addr(mut other_nodes: Vec<String>, socket: UdpSocket) -> Vec<String> {
    loop {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();
        let msg_addr = Encoder::decode_from_bytes(buf.to_vec());
        if msg_addr == END {
            break;
        }
        if !&other_nodes.contains(&msg_addr) {
            other_nodes.push(msg_addr.to_string());
        }
    }
    other_nodes
}

pub fn run_bully_as_non_leader(mut blockchain: Blockchain) {
    // Let the OS to pick one addr + port for us
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let mut other_nodes: Vec<String> = vec![];

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("Enviando mensaje {} al lider", DUMMY_MSG);
    socket
        .send_to(&Encoder::encode_to_bytes(REGISTER_MSG), LEADER_ADDR)
        .unwrap();

    socket
        .send_to(&Encoder::encode_to_bytes(DUMMY_MSG), LEADER_ADDR)
        .unwrap();

    for _ in 0..10 {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();

        let msg = Encoder::decode_from_bytes(buf.to_vec());
        println!("{:?}", other_nodes);

        if msg.as_str() == NEW_NODE {
            other_nodes = recv_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
        } else {
            println!("Recibido {}", &msg);
        }

        blockchain.add(Block { data: msg });
    }

    println!("Blockchain final: {:?}", blockchain);
}
