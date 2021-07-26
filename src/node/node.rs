use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use crate::messages::*;
use crate::record::{Record, RecordData};

const CTOR_ADDR: &str = "127.0.0.1:8001";

pub struct Node {
    pub writer: TcpStream,
    pub reader: BufReader<TcpStream>,
    pub bully_sock: UdpSocket,
    pub leader_addr: Arc<Mutex<Option<String>>>,
    pub blockchain: Blockchain,
}

impl Node {
    pub fn new() -> Self {
        let stream = TcpStream::connect(CTOR_ADDR).unwrap();
        let writer = stream.try_clone().unwrap();
        let reader = BufReader::new(stream);
        let blockchain = Blockchain::new();
        let bully_sock = UdpSocket::bind("0.0.0.0:0").unwrap();
        let leader_addr = Arc::new(Mutex::new(None));
        Node {
            writer,
            reader,
            leader_addr,
            bully_sock,
            blockchain,
        }
    }

    pub fn run(&mut self, stdin_buf: Arc<Mutex<Option<String>>>) -> () {
        self.fetch_leader_addr();

        // FIXME: usar condvars
        while self.leader_addr.lock().unwrap().is_none() {
            std::thread::sleep(Duration::from_secs(1));
        }

        let leader_addr = (*self.leader_addr.lock().unwrap())
            .clone()
            .expect("No leader addr");

        println!("Leader address: {}", leader_addr);

        let i_am_leader = leader_addr == self.bully_sock.local_addr().unwrap().to_string();

        match i_am_leader {
            true => self.run_bully_as_leader(stdin_buf),
            false => self.run_bully_as_non_leader(leader_addr, stdin_buf),
        }
    }

    pub fn run_bully_as_non_leader(
        &mut self,
        leader_addr: String,
        stdin_buf: Arc<Mutex<Option<String>>>,
    ) {
        let mut other_nodes: Vec<String> = vec![];

        let mysocket = self.bully_sock.try_clone().unwrap();
        let tmp = leader_addr.clone();

        //TODO. sacar este busy wait reemplazarlo por condvar
        thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let value = (*stdin_buf.lock().unwrap()).clone();
            match value {
                Some(stdin_msg) => {
                    mysocket
                        .send_to(&encode_to_bytes(&stdin_msg), tmp.as_str())
                        .unwrap();
                    *(&stdin_buf).lock().unwrap() = None;
                }
                _ => {}
            }
        });

        self.bully_sock
            .send_to(&encode_to_bytes(REGISTER_MSG), leader_addr.as_str())
            .unwrap();

        loop {
            let (msg, _from)= self.read_from();
            match msg.as_str() {
                NEW_NODE => {
                    other_nodes = self.recv_all_addr(other_nodes.clone(), self.bully_sock.try_clone().unwrap());
                    println!("{:?}", other_nodes);
                }
                BLOCKCHAIN => {
                    self.blockchain = self.recv_blockchain(self.bully_sock.try_clone().unwrap());
                }
                CLOSE => {
                    break;
                }
                msg => {
                    let record = self.create_record(msg, self.bully_sock.try_clone().unwrap().local_addr().unwrap());
                    let mut block = Block::new(self.blockchain.get_last_block_hash());
                    block.add_record(record);
                    if let Err(err) = self.blockchain.append_block(block)
                    {
                        println!("{}", err);
                    }
                    else {
                        println!("{}", self.blockchain);
                    }
                }
            }
        }
        println!("{}", self.blockchain);
        println!("Desconectando...");
        process::exit(-1);
    }

    pub fn run_bully_as_leader(&mut self, _stdin_buf: Arc<Mutex<Option<String>>>) {
        println!("Soy el líder!");
        let mut other_nodes: Vec<SocketAddr> = vec![];
        let mut propagated_msgs = 0;

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
            let (msg, from)= self.read_from();
            if propagated_msgs == 10 {
                break;
            }
            match msg.as_str() {
                REGISTER_MSG => {
                    println!("Registrando nodo: {}", from);
                    if !&other_nodes.contains(&from) {
                        other_nodes.push(from);
                        self.send_all_addr(other_nodes.clone(),self.bully_sock.try_clone().unwrap());
                        self.send_blockchain(self.blockchain.clone(),from,self.bully_sock.try_clone().unwrap());
                    }
                    println!("{:?}", other_nodes);
                }
                CLOSE => {
                    self.bully_sock.send_to(&encode_to_bytes(&msg), from).unwrap();
                    other_nodes.retain(|&x| x != from);
                    self.send_all_addr(other_nodes.clone(), self.bully_sock.try_clone().unwrap());
                }
                msg => {
                    println!("Propagando cambios {:?} al resto de los nodos", msg);
                    for node in &other_nodes {
                        self.bully_sock.send_to(&encode_to_bytes(msg), node).unwrap();
                    }
                    let mut block = Block::new(self.blockchain.get_last_block_hash());
                    block.add_record(self.create_record(msg, from));

                    if let Err(err) = self.blockchain.append_block(block)
                    {
                        println!("{}", err);
                    }
                    else {
                        println!("{}", self.blockchain);
                    }
                    propagated_msgs += 1;
                }
            }
        }
    }

    fn read_from(&self) -> (String, SocketAddr) {
        let mut buf = [0; 128];
        let (_, from) = self.bully_sock.recv_from(&mut buf).unwrap();
        let msg = decode_from_bytes(buf.to_vec());
        (msg, from)
    }

    fn _acquire(&mut self) {
        self.writer.write_all(ACQUIRE_MSG.as_bytes()).unwrap();
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        println!("Read {}", buffer);
    }

    fn _release(&mut self) {
        self.writer.write_all(RELEASE_MSG.as_bytes()).unwrap();
    }

    /// Contacts the coordinator to get the current leader_addr
    fn fetch_leader_addr(&mut self) {
        println!("Enviando mensaje de discovery");


        // Sends a NEW_NODE_MSG to the coordinator
        self.writer.write_all(NEW_NODE_MSG.as_bytes()).unwrap();
        // Sends the bully_addr to the coordinator so that he can use it if no leader exists yet
        self.writer
            .write_all(&encode_to_bytes(
                self.bully_sock.local_addr().unwrap().to_string().as_str(),
            ))
            .unwrap();

        // The coordinator will answer with the bully_addr of the leader (mine or other)
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        let leader_ip = buffer.split('\n').collect::<Vec<&str>>()[0].to_string();

        (*self.leader_addr.lock().unwrap()) = Some(leader_ip);
    }

    fn send_blockchain(&self, blockchain: Blockchain, from: SocketAddr, socket: UdpSocket) {
        socket.send_to(&encode_to_bytes(BLOCKCHAIN), from).unwrap();
        for b in blockchain.blocks {
            let mut data_to_send = String::new();
            match &b.records[0].record {
                RecordData::CreateStudent(id, qualification) => {
                    data_to_send.push_str(
                        &(format!(
                            "{},{},{},{}",
                            &id,
                            &(qualification.to_string()),
                            &b.records[0].created_at.as_millis().to_string(),
                            &b.records[0].from
                        )),
                    );
                }
            };

            socket
                .send_to(&encode_to_bytes(&data_to_send), from)
                .unwrap();
        }
        socket.send_to(&encode_to_bytes(END), from).unwrap();
    }

    fn recv_blockchain(&self, socket: UdpSocket) -> Blockchain {
        let mut blockchain = Blockchain::new();
        loop {
            let mut buf = [0; 128];
            let (_, _) = socket.recv_from(&mut buf).unwrap();
            let msg = decode_from_bytes(buf.to_vec());
            if msg == END {
                break;
            }
            let mut block = Block::new(blockchain.get_last_block_hash());
            block.add_record(self.read_record(msg));
            if let Err(err) = blockchain.append_block(block)
            {
                println!("{}", err);
            }
            else {
                println!("{}", blockchain);
            }
        }
        blockchain
    }

    fn send_all_addr(&self, other_nodes: Vec<SocketAddr>, socket: UdpSocket) {
        for node_conected in &other_nodes {
            socket
                .send_to(&encode_to_bytes(NEW_NODE), node_conected)
                .unwrap();
            for node_addr in &other_nodes {
                if node_addr != node_conected {
                    // No queremos mandar la propia IP a cada nodo
                    continue;
                }
                let addr = format!("{}", node_addr);
                socket
                    .send_to(&encode_to_bytes(&addr), node_conected)
                    .unwrap();
            }
            socket
                .send_to(&encode_to_bytes(END), node_conected)
                .unwrap();
        }
    }

    fn recv_all_addr(&self, mut other_nodes: Vec<String>, socket: UdpSocket) -> Vec<String> {
        loop {
            let mut buf = [0; 128];
            let (_, _) = socket.recv_from(&mut buf).unwrap();
            let msg_addr = decode_from_bytes(buf.to_vec());
            if msg_addr == END {
                break;
            }
            if !&other_nodes.contains(&msg_addr) {
                other_nodes.push(msg_addr.to_string());
            }
        }
        other_nodes
    }

    fn read_record(&self, msg: String) -> Record {
        let student_data: Vec<&str> = msg.split(",").collect();
        let record = Record::new(
            student_data[3].into(),
            RecordData::CreateStudent(
                student_data[0].into(),
                student_data[1].parse::<u32>().unwrap(),
            ),
            Duration::from_millis(student_data[2].parse::<u64>().unwrap()),
        );
        record
    }

    fn create_record(&self, msg: &str, from: SocketAddr) -> Record {
        let student_data: Vec<&str> = msg.split(",").collect();
        let record = Record::new(
            from.to_string().into(),
            RecordData::CreateStudent(
                student_data[0].into(),
                student_data[1].parse::<u32>().unwrap(),
            ),
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
        );
        record
    }
}
