use crate::encoder::Encoder;

use std::str;

use crate::blockchain::{Block, Blockchain};
use std::net::UdpSocket;

const DUMMY_MSG: &str = "ping";
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

pub fn run_bully_as_non_leader(socket: UdpSocket, mut blockchain: Blockchain, leader_addr: String) {
    // Let the OS to pick one addr + port for us
    let mut other_nodes: Vec<String> = vec![];

    println!("Enviando mensaje {} al lider", DUMMY_MSG);
    socket
        .send_to(&Encoder::encode_to_bytes(REGISTER_MSG), leader_addr.as_str())
        .unwrap();

    socket
        .send_to(&Encoder::encode_to_bytes(DUMMY_MSG), leader_addr.as_str())
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
