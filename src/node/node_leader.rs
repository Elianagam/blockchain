
use crate::encoder::Encoder;
use std::net::{SocketAddr, UdpSocket};
use std::str;

use crate::blockchain::{Block, Blockchain};

const LEADER_ADDR: &str = "127.0.0.1:8000";
const REGISTER_MSG: &str = "register";

pub fn run_bully_as_leader(mut blockchain: Blockchain) {
    let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

    println!("Soy el líder!");
    let mut other_nodes: Vec<SocketAddr> = vec![];

    let mut propagated_msgs = 0;

    loop {
        let mut buf = [0; 128];
        let (_, from) = socket.recv_from(&mut buf).unwrap();

        let msg = Encoder::decode_from_bytes(buf.to_vec());

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
