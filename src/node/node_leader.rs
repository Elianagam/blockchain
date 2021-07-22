
use crate::encoder::Encoder;
use std::net::{SocketAddr, UdpSocket};
use std::str;

use crate::blockchain::{Block, Blockchain};

const LEADER_ADDR: &str = "127.0.0.1:8000";
const REGISTER_MSG: &str = "register";
const NEW_NODE: &str = "new_node";
const END: &str = "-";

fn send_all_addr(other_nodes: Vec<SocketAddr>, from: SocketAddr, socket: UdpSocket) {
    socket.send_to(&Encoder::encode_to_bytes(NEW_NODE), from).unwrap();
    for node in &other_nodes {
        let addr = format!("{}", node);
        socket.send_to(&Encoder::encode_to_bytes(&addr), from).unwrap();
    }
    socket.send_to(&Encoder::encode_to_bytes(END), from).unwrap();
}

pub fn run_bully_as_leader(mut blockchain: Blockchain) {
    let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

    println!("Soy el líder!");
    let mut other_nodes: Vec<SocketAddr> = vec![];

    let mut propagated_msgs = 0;

    loop {
        let mut buf = [0; 128];
        let (_, from) = socket.recv_from(&mut buf).unwrap();

        let msg = Encoder::decode_from_bytes(buf.to_vec());

        if propagated_msgs == 10 {
            break;
        }
        println!("{:?}", other_nodes);

        match msg.as_str() {
            REGISTER_MSG => {
                println!("Registrando nodo: {}", from);
                if !&other_nodes.contains(&from) {
                    other_nodes.push(from);
                }

            }
            msg => {
                println!("Propagando cambios {:?} al resto de los nodos", msg);
                send_all_addr(other_nodes.clone(), from, socket.try_clone().unwrap());
                for node in &other_nodes {
                    socket.send_to(&Encoder::encode_to_bytes(msg), node).unwrap();

                }
                blockchain.add(Block {
                    data: msg.to_string(),
                });
                propagated_msgs += 1;
            }
        }
    }
}
