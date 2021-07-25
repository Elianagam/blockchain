use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::blockchain::{Block, Blockchain};
use crate::encoder::Encoder;
use crate::messages::{BLOCKCHAIN, CLOSE, END, NEW_NODE, NEW_NODE_MSG, PING_MSG, REGISTER_MSG};
use std::net::{SocketAddr, UdpSocket};

const CTOR_ADDR: &str = "127.0.0.1:8001";

pub struct Node {
    pub writer: TcpStream,
    pub reader: BufReader<TcpStream>,
    pub leader_addr: Arc<Mutex<Option<String>>>,
    pub bully_addr: String,
}

impl Node {
    pub fn new(bully_addr: String, leader_addr: Arc<Mutex<Option<String>>>) -> Self {
        let stream = TcpStream::connect(CTOR_ADDR).unwrap();
        let writer = stream.try_clone().unwrap();
        let reader = BufReader::new(stream);
        Node {
            writer,
            reader,
            leader_addr,
            bully_addr,
        }
    }

    pub fn run(
        &mut self,
        bully_sock: UdpSocket,
        leader_addr: Arc<Mutex<Option<String>>>,
        stdin_buf: Arc<Mutex<Option<String>>>,
    ) -> () {
        self.fetch_leader_addr();
        let blockchain = Blockchain::new();

        // FIXME: usar condvars
        while leader_addr.lock().unwrap().is_none() {
            std::thread::sleep(Duration::from_secs(1));
        }

        let leader_addr = (*leader_addr.lock().unwrap())
            .clone()
            .expect("No leader addr");

        let iamleader = leader_addr == bully_sock.local_addr().unwrap().to_string();

        match iamleader {
            true => self.run_bully_as_leader(bully_sock, blockchain, stdin_buf),
            false => self.run_bully_as_non_leader(bully_sock, blockchain, leader_addr, stdin_buf),
        }
    }

    pub fn run_bully_as_non_leader(
        &self,
        socket: UdpSocket,
        mut blockchain: Blockchain,
        leader_addr: String,
        stdin_buf: Arc<Mutex<Option<String>>>,
    ) {
        let mut other_nodes: Vec<String> = vec![];

        let mysocket = socket.try_clone().unwrap();
        let tmp = leader_addr.clone();

        //TODO. sacar este busy wait reemplazarlo por condvar
        thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let value = (*stdin_buf.lock().unwrap()).clone();
            match value {
                Some(stdin_msg) => {
                    mysocket
                        .send_to(&Encoder::encode_to_bytes(&stdin_msg), tmp.as_str())
                        .unwrap();
                    *(&stdin_buf).lock().unwrap() = None;
                }
                _ => {}
            }
        });

        socket
            .send_to(
                &Encoder::encode_to_bytes(REGISTER_MSG),
                leader_addr.as_str(),
            )
            .unwrap();
        socket
            .send_to(&Encoder::encode_to_bytes(PING_MSG), leader_addr.as_str())
            .unwrap();

        loop {
            let mut buf = [0; 128];
            let (_, _) = socket.recv_from(&mut buf).unwrap();
            let msg = Encoder::decode_from_bytes(buf.to_vec());
            println!("{:?}", other_nodes);

            match msg.as_str() {
                NEW_NODE => {
                    other_nodes =
                        self.recv_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
                }
                BLOCKCHAIN => {
                    blockchain = self.recv_blockchain(blockchain, socket.try_clone().unwrap());
                }
                CLOSE => {
                    break;
                }
                msg => {
                    println!("Recibido {}", &msg);
                    blockchain.add(Block {
                        data: msg.to_string(),
                    });
                    println!("{}", blockchain);
                }
            }
        }
        println!("{}\nNodo Desconectado...", blockchain);
        process::exit(-1);
    }

    pub fn run_bully_as_leader(
        &self,
        socket: UdpSocket,
        mut blockchain: Blockchain,
        stdin_buf: Arc<Mutex<Option<String>>>,
    ) {
        println!("Soy el líder!");
        let mut other_nodes: Vec<SocketAddr> = vec![];

        /*
        let value = (*stdin_buf.lock().unwrap()).clone();
            match value {
                Some(stdin) => {
                    match stdin.as_str() {
                        CLOSE => { break; }
                        msg => {
                            blockchain = self.add_block(
                                &msg,
                                blockchain.clone(),
                                other_nodes.clone(),
                                socket.try_clone().unwrap(),
                            );
                        }
                    }
                }
                None => {}
            }
        */
        loop {
            let mut buf = [0; 128];
            let (_, from) = socket.recv_from(&mut buf).unwrap();
            let msg = Encoder::decode_from_bytes(buf.to_vec());

            println!("{:?}", other_nodes);

            match msg.as_str() {
                REGISTER_MSG => {
                    println!("Registrando nodo: {}", from);
                    if !&other_nodes.contains(&from) {
                        other_nodes.push(from);
                        self.send_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
                        self.send_blockchain(blockchain.clone(), from, socket.try_clone().unwrap());
                    }
                }
                CLOSE => {
                    socket
                        .send_to(&Encoder::encode_to_bytes(&msg), from)
                        .unwrap();
                    other_nodes.retain(|&x| x != from);
                    self.send_all_addr(other_nodes.clone(), socket.try_clone().unwrap());
                }
                msg => {
                    blockchain = self.add_block(
                        msg,
                        blockchain.clone(),
                        other_nodes.clone(),
                        socket.try_clone().unwrap(),
                    );
                }
            }
        }
    }

    fn add_block(
        &self,
        msg: &str,
        mut blockchain: Blockchain,
        other_nodes: Vec<SocketAddr>,
        socket: UdpSocket,
    ) -> Blockchain {
        println!("Propagando cambios {:?} al resto de los nodos", msg);
        for node in &other_nodes {
            socket
                .send_to(&Encoder::encode_to_bytes(msg), node)
                .unwrap();
        }
        blockchain.add(Block {
            data: msg.to_string(),
        });
        blockchain
    }

    /*
    fn acquire(&mut self) {
        self.writer.write_all(ACQUIRE_MSG.as_bytes()).unwrap();
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        println!("Read {}", buffer);
    }

    fn release(&mut self) {
        self.writer.write_all(RELEASE_MSG.as_bytes()).unwrap();
    }
    */

    fn fetch_leader_addr(&mut self) {
        // Pregunta al coordinador la IP del lider actual, si recibimos
        // nuestra IP entonces somos nosotros.
        println!("Enviando mensaje de discovery");

        self.writer.write_all(NEW_NODE_MSG.as_bytes()).unwrap();
        self.writer
            .write_all(&Encoder::encode_to_bytes(self.bully_addr.as_str()))
            .unwrap();

        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        let leader_ip = buffer.split('\n').collect::<Vec<&str>>()[0].to_string();

        (*self.leader_addr.lock().unwrap()) = Some(leader_ip);
    }

    fn send_blockchain(&self, blockchain: Blockchain, from: SocketAddr, socket: UdpSocket) {
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

    fn recv_blockchain(&self, mut blockchain: Blockchain, socket: UdpSocket) -> Blockchain {
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

    fn send_all_addr(&self, other_nodes: Vec<SocketAddr>, socket: UdpSocket) {
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

    fn recv_all_addr(&self, mut other_nodes: Vec<String>, socket: UdpSocket) -> Vec<String> {
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
}
