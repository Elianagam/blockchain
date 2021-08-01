use crate::blockchain::blockchain::Blockchain;
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use crate::leader_discoverer::LeaderDiscoverer;
use crate::leader_down::LeaderDown;
use crate::utils::messages::*;
use crate::blockchain::record::{Record,RecordData};
use crate::blockchain::block::Block;
use crate::stdin_reader::StdinReader;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration,SystemTime};

const MAX_NODES: u32 = 50;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: UdpSocket,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub alive: Arc<RwLock<bool>>,
    pub leader_down_convar: Arc<(Mutex<bool>, Condvar)>,
}

fn build_addr_list(skip_addr: &String) -> Vec<String> {
    let mut addrs = vec![];
    for i in 0..MAX_NODES {
        let addr = format!("127.0.0.1:{}", 8000 + i);
        if &addr != skip_addr {
            addrs.push(addr);
        }
    }
    addrs
}

impl Node {
    pub fn new(port_number: &str) -> Self {
        let port_number = port_number.parse::<u32>().unwrap();
        if port_number < 8000 || port_number > (8000 + MAX_NODES) {
            panic!(
                "Port number must be between {} and {}",
                8000,
                8000 + MAX_NODES
            );
        }
        let my_address: String = format!("127.0.0.1:{}", port_number);

        let other_nodes = Arc::new(build_addr_list(&my_address));

        Node {
            my_address: Arc::new(RwLock::new(my_address.clone())),
            socket: UdpSocket::bind(my_address.clone()).unwrap(),
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Blockchain::new(),
            leader_condvar: Arc::new((Mutex::new(false), Condvar::new())),
            other_nodes,
            alive: Arc::new(RwLock::new(true)),
            leader_down_convar: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    pub fn run(&mut self) -> () {
        println!(
            "Running node on: {} ",
            self.socket.local_addr().unwrap().to_string()
        );

        self.discover_leader();

        let clone_socket = self.socket.try_clone().unwrap();   
        let leader_addr_clone = self.leader_addr.clone();
        let alive_clone = self.alive.clone();
        let cv_clone = self.leader_condvar.clone();

        thread::spawn(move || {
            let (lock, cv) = &*cv_clone;
            let mut leader_found = lock.lock().unwrap();
            while !*leader_found {
                leader_found = cv.wait(leader_found).unwrap();
            }

            let leader_addr = (leader_addr_clone.read().unwrap()).clone();
            let reader = StdinReader::new(clone_socket, leader_addr, alive_clone);
            reader.run();
        });

        while *self.alive.read().unwrap() {
            let (msg, from)= self.read_from();
            match msg.as_str() {
                WHO_IS_LEADER => {
                    if self.i_know_the_leader() {
                        self.check_if_i_am_leader(from.to_string());
                    }
                }
                I_AM_LEADER => {
                    self.leader_found(from);
                }
                BLOCKCHAIN => {
                    self.blockchain = self.recv_blockchain();
                    println!("{}", self.blockchain);
                }
                LEADER_IS_DOWN => {
                    if !self.i_am_leader() {
                        println!("Se cayo el leader!! Que hago???")
                    }
                }
                msg => {
                    let record = self.create_record(msg, from);
                    let mut block = Block::new(self.blockchain.get_last_block_hash());
                    block.add_record(record);
                    if let Err(err) = self.blockchain.append_block(block) {
                        println!("Error: {}", err);
                    } 
                    if self.i_am_leader() {
                        // Si el mensaje viene del leader, lo propago a todos
                        for node in &*self.other_nodes {
                            self.socket.send_to(&encode_to_bytes(msg), node).unwrap();
                        }
                    } else {
                        self.ask_leader_down();
                    }
                    println!("{}", self.blockchain);
                }
            }
        }
        println!("Desconectando...");
    }

    fn ask_leader_down(&self) -> () {
        let mut leader_down = LeaderDown::new(
            self.leader_down_convar.clone(),
            self.leader_addr.clone(),
            self.socket.try_clone().expect("Error cloning socket"),
            self.other_nodes.clone(),
        );

        thread::spawn(move || {
            leader_down.run();
        });
    }

    fn discover_leader(&self) -> () {
        let mut leader_discoverer = LeaderDiscoverer::new(
            self.leader_condvar.clone(),
            self.leader_addr.clone(),
            self.my_address.clone(),
            self.socket.try_clone().expect("Error cloning socket"),
            self.other_nodes.clone(),
        );

        thread::spawn(move || {
            leader_discoverer.run();
        });
    }

    fn i_know_the_leader(&self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        } else {
            println!("Leader is being changed");
            return false;
        }
    }

    fn read_from(&self) -> (String, SocketAddr) {
        let mut buf = [0; 128];
        let (_, from) = self.socket.recv_from(&mut buf).unwrap();
        let msg = decode_from_bytes(buf.to_vec());
        (msg, from)
    }

    fn i_am_leader(&self) -> bool {
        println!("my address: {} ", self.my_address.read().unwrap());
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None {
                return false;
            }
            return *self.my_address.read().unwrap() == *leader_addr_mut.clone().unwrap();
        } else {
            // FIXME
            println!("not implemented");
            return false;
        }
    }

    // Este metodo se ejecuta cuando se setea un lider externo
    // como lider. 
    fn leader_found(&self, leader: SocketAddr) -> () {
        let (lock, cvar) = &*self.leader_condvar;
        let mut leader_found = lock.lock().unwrap();
        *leader_found = true;
        cvar.notify_all();

        if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
            *leader_addr_mut = Some(leader.to_string());
        }
        let mut leader_addr = (*self.leader_addr.read().unwrap()).clone();
        println!(
            "La direccion del leader es: {}",
            leader_addr.get_or_insert("No address".to_string())
        );
    }

    fn check_if_i_am_leader(&self, node_that_asked: String) -> () {
        println!("Checking if I'm leader");
        if self.i_am_leader() {
            print!(
                "Sending to {} that I am leader",
                node_that_asked.to_string()
            );
            self.socket
                .send_to(&encode_to_bytes(I_AM_LEADER), node_that_asked.clone())
                .unwrap();
            self.send_blockchain(node_that_asked.clone());
        } else {
            println!("I am not leader");
        }
    }

    fn send_blockchain(&self, from: String) {
        self.socket.send_to(&encode_to_bytes(BLOCKCHAIN), from.clone()).unwrap();
        for b in self.blockchain.get_blocks() {
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

            self.socket
                .send_to(&encode_to_bytes(&data_to_send), from.clone().to_string())
                .unwrap();
        }
        self.socket.send_to(&encode_to_bytes(END), from).unwrap();
    }

    fn recv_blockchain(&self) -> Blockchain {
        let mut blockchain = Blockchain::new();
        loop {
            let (msg, from) = self.read_from();
            if msg == END {
                break;
            }
            let mut block = Block::new(blockchain.get_last_block_hash());
            block.add_record(self.read_record(msg, from));
            if let Err(err) = blockchain.append_block(block) {
                println!("{}", err);
            } 
        }
        blockchain
    }

    fn read_record(&self, msg: String, from: SocketAddr) -> Record {
        let student_data: Vec<&str> = msg.split(",").collect();
        let record = Record::new(
            from.to_string().into(),
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
