use crate::encoder::Encoder;
use crate::messages::{REGISTER_MSG, NEW_NODE, END};

use std::net::{SocketAddr, UdpSocket};

use crate::blockchain::{Block, Blockchain};

fn send_all_addr(other_nodes: Vec<SocketAddr>, socket: UdpSocket) {
    for node_conected in &other_nodes {
        socket
            .send_to(&Encoder::encode_to_bytes(NEW_NODE), node_conected)
            .unwrap();
        for node_addr in &other_nodes {
            let addr = format!("{}", node_addr);
            socket
                .send_to(&Encoder::encode_to_bytes(&addr), node_conected)
                .unwrap();
        }
        socket
            .send_to(&Encoder::encode_to_bytes(END), node_conected)
            .unwrap();
    }
}

pub fn run_bully_as_leader(socket: UdpSocket, mut blockchain: Blockchain) {
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

        match msg.as_str() {
            REGISTER_MSG => {
                println!("Registrando nodo: {}", from);
                if !&other_nodes.contains(&from) {
                    other_nodes.push(from);
                    send_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
                }
            }
            NEW_NODE => {
                // TODO: SEND BLOCKCHAIN
            }
            msg => {
                println!("Propagando cambios {:?} al resto de los nodos", msg);
                for node in &other_nodes {
                    socket
                        .send_to(&Encoder::encode_to_bytes(msg), node)
                        .unwrap();
                }
                blockchain.add(Block {
                    data: msg.to_string(),
                });
                propagated_msgs += 1;

            }
        }
    }
}
