use crate::messages::{PING_MSG, REGISTER_MSG, NEW_NODE, END, BLOCKCHAIN_MSG};
use crate::encoder::Encoder;
use crate::blockchain::{Block, Blockchain};

use std::net::UdpSocket;
use std::process;
use std::io::{self, BufRead};

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

fn read_stdin() -> String {
    println!("Ingresar dato a blockchain: ");
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    let line = iterator.next().unwrap().unwrap();
    line
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
    loop {
        let mut buf = [0; 128];
        let (_, _) = socket.recv_from(&mut buf).unwrap();
        let msg = Encoder::decode_from_bytes(buf.to_vec());
        

        match msg.as_str() {
            NEW_NODE =>{
                other_nodes = recv_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
            }
            BLOCKCHAIN_MSG => {
                blockchain = recv_blockchain(blockchain, socket.try_clone().unwrap());
            }
            "close" => { 
                break;
            }
            msg => {
                println!("Recibido {}", &msg);
                blockchain.add(Block { data: msg.to_string() });
                socket
                    .send_to(&Encoder::encode_to_bytes(&read_stdin()), leader_addr.as_str())
                    .unwrap();
                println!("{}", blockchain);
            }
        }
    }
    println!("{}\nNodo Desconectado...", blockchain);
    process::exit(-1);
}
