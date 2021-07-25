use crate::encoder::Encoder;
use crate::messages::{BLOCKCHAIN, CLOSE, END, NEW_NODE, REGISTER_MSG};

use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;


use crate::blockchain::{Block, Blockchain};

fn send_all_addr(other_nodes: Vec<SocketAddr>, socket: UdpSocket) {
    for node_conected in &other_nodes {
        socket
            .send_to(&Encoder::encode_to_bytes(NEW_NODE), node_conected)
            .unwrap();
        for node_addr in &other_nodes {
            if node_addr == node_conected {
                // No queremos mandar la propia IP a cada nodo
                continue;
            }
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

fn send_blockchain(blockchain: Blockchain, from: SocketAddr, socket: UdpSocket) {
    socket
        .send_to(&Encoder::encode_to_bytes(BLOCKCHAIN), from)
        .unwrap();
    for b in blockchain.get_blocks() {
        socket
            .send_to(&Encoder::encode_to_bytes(&b.data), from)
            .unwrap();
    }
    socket
        .send_to(&Encoder::encode_to_bytes(END), from)
        .unwrap();
}

pub fn run_bully_as_leader(
    socket: UdpSocket,
    mut blockchain: Blockchain,
    _stdin_buf: Arc<Mutex<Option<String>>>,
) {
    println!("Soy el líder!");
    let mut other_nodes: Vec<SocketAddr> = vec![];
    let mut propagated_msgs = 0;
    //let _clone_other_nodes = other_nodes.clone();
    //let _clone_socket = socket.try_clone().unwrap();

    loop {
        let mut buf = [0; 128];
        let (_, from) = socket.recv_from(&mut buf).unwrap();
        let msg = Encoder::decode_from_bytes(buf.to_vec());

        if propagated_msgs == 10 { break; }

        match msg.as_str() {
            REGISTER_MSG => {
                println!("Registrando nodo: {}", from);
                if !&other_nodes.contains(&from) {
                    other_nodes.push(from);
                    send_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
                    send_blockchain(blockchain.clone(), from, socket.try_clone().unwrap());
                }
            }
            CLOSE => {
                socket
                    .send_to(&Encoder::encode_to_bytes(&msg), from)
                    .unwrap();
                other_nodes.retain(|&x| x != from);
                send_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
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
