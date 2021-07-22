use crate::encoder::encode_to_bytes;

use crate::encoder::decode_from_bytes;
use std::str;

use crate::blockchain::{Block, Blockchain};
use std::net::UdpSocket;

const LEADER_ADDR: &str = "127.0.0.1:8000";
const DUMMY_MSG: &str = "testing";
const REGISTER_MSG: &str = "register";

pub fn run_bully_as_non_leader(mut blockchain: Blockchain) {
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
