use crate::messages::{PING_MSG, REGISTER_MSG, NEW_NODE, END, BLOCKCHAIN_MSG};
use crate::encoder::Encoder;
use crate::blockchain::{Block, Blockchain};

use std::net::UdpSocket;

fn recv_blockchain(mut blockchain: Blockchain, socket: UdpSocket) -> Blockchain {
    loop {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();
        let block = Encoder::decode_from_bytes(buf.to_vec());
        if block == END {
            break;
        }
        blockchain.add(Block { data: block });
    }
    blockchain
}

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
    let mut other_nodes: Vec<String> = vec![];

    println!("Enviando mensaje {} al lider", PING_MSG);
    socket
        .send_to(&Encoder::encode_to_bytes(REGISTER_MSG), leader_addr.as_str())
        .unwrap();

    socket
        .send_to(&Encoder::encode_to_bytes(PING_MSG), leader_addr.as_str())
        .unwrap();

    for _ in 0..10 {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();

        let msg = Encoder::decode_from_bytes(buf.to_vec());
        println!("{:?}", other_nodes);

        match msg.as_str() {
            NEW_NODE =>{
                other_nodes = recv_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
            }
            BLOCKCHAIN_MSG => {
                blockchain = recv_blockchain(blockchain, socket.try_clone().unwrap());
            }
            msg => {
                println!("Recibido {}", &msg);
                blockchain.add(Block { data: msg.to_string() });
                println!("{}", blockchain);
            }
        }
    }

    println!("Blockchain final:\n\t{}", blockchain);
    
}
