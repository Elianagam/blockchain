use crate::blockchain::block::Block;
use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::record::{Record, RecordData};
use crate::leader_discoverer::LeaderDiscoverer;
use crate::leader_down_handler::LeaderDownHandler;
use crate::stdin_reader::StdinReader;
use crate::utils::messages::*;
use crate::utils::socket_with_timeout::SocketWithTimeout;

use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

const MAX_NODES: u32 = 50;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: SocketWithTimeout,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub election_condvar: Arc<(Mutex<Option<String>>, Condvar)>,

    // El nodo esta vivo (no se hizo `close`)
    pub alive: Arc<RwLock<bool>>,

    // Convar para detectar mensajes ack
    pub msg_ack_cv: Arc<(Mutex<bool>, Condvar)>,

    pub leader_down: Arc<(Mutex<bool>, Condvar)>,
    pub running_bully: Arc<Mutex<bool>>,
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
        let socket = UdpSocket::bind(my_address.clone()).unwrap();

        Node {
            my_address: Arc::new(RwLock::new(my_address.clone())),
            socket: SocketWithTimeout::new(socket),
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Blockchain::new(),
            leader_condvar: Arc::new((Mutex::new(false), Condvar::new())),
            election_condvar: Arc::new((Mutex::new(None), Condvar::new())),
            alive: Arc::new(RwLock::new(true)),
            msg_ack_cv: Arc::new((Mutex::new(false), Condvar::new())),
            leader_down: Arc::new((Mutex::new(false), Condvar::new())),
            running_bully: Arc::new(Mutex::new(false)),
            other_nodes,
        }
    }

    pub fn run(&mut self) -> () {
        println!("Running node on: {} ", self.socket.local_addr().to_string());

        self.discover_leader();
        self.detect_if_leader_is_down();
        self.stdin_reader();

        while *self.alive.read().unwrap() {
            let (_, from, msg) = self.socket.recv_from();
            match msg.as_str() {
                WHO_IS_LEADER => {
                    if self.i_know_the_leader() {
                        self.check_if_i_am_leader(from.to_string());
                    }
                }
                COORDINATOR => {
                    self.handle_coordinator_msg(from);
                }
                BLOCKCHAIN => {
                    self.blockchain = self.recv_blockchain();
                    println!("{}", self.blockchain);
                }
                OK => {
                    // Basicamente cada vez que recibamos un mensaje le hacemos un notify
                    // a la otra convar y seteamos la IP del que recibimos.
                    let (lock, cvar) = &*self.election_condvar;
                    *lock.lock().unwrap() = Some(from.to_string());
                    cvar.notify_all();
                }
                ELECTION => {
                    self.socket
                        .send_to(OK.to_string(), from.to_string())
                        .unwrap();

                    let (lock, cvar) = &*self.leader_down;
                    *lock.lock().unwrap() = true;
                    cvar.notify_all();
                }
                ACK_MSG => {
                    let (lock, cv) = &*self.msg_ack_cv;
                    *lock.lock().unwrap() = true;
                    cv.notify_all();
                }
                msg => {
                    let record = self.create_record(msg, from);
                    let mut block = Block::new(self.blockchain.get_last_block_hash());
                    block.add_record(record);
                    if let Err(err) = self.blockchain.append_block(block) {
                        println!("Error: {}", err);
                    }
                    if self.i_am_leader() {
                        self.socket
                            .send_to(ACK_MSG.to_string(), from.to_string())
                            .unwrap();
                        // Si el mensaje viene del leader, lo propago a todos
                        for node in &*self.other_nodes {
                            self.socket.send_to(msg.to_string(), node.clone()).unwrap();
                        }
                    }
                    println!("{}", self.blockchain);
                }
            }
        }
    }

    fn stdin_reader(&mut self) {
        let mut reader = StdinReader::new(
            self.leader_condvar.clone(),
            self.socket.try_clone(),
            self.leader_addr.clone(),
            self.alive.clone(),
            self.msg_ack_cv.clone(),
            self.leader_down.clone(),
        );

        thread::spawn(move || {
            reader.run();
        });
    }

    fn discover_leader(&mut self) -> () {
        let mut leader_discoverer = LeaderDiscoverer::new(
            self.leader_condvar.clone(),
            self.leader_addr.clone(),
            self.my_address.clone(),
            self.socket.try_clone(),
            self.other_nodes.clone(),
        );

        thread::spawn(move || {
            leader_discoverer.run();
        });
    }

    fn detect_if_leader_is_down(&mut self) -> () {
        let mut leader_down_handler = LeaderDownHandler::new(
            self.my_address.clone(),
            self.socket.try_clone(),
            self.election_condvar.clone(),
            self.leader_down.clone(),
            self.running_bully.clone(),
        );

        thread::spawn(move || {
            leader_down_handler.run();
        });
    }

    fn i_know_the_leader(&mut self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        }
        false
    }

    fn i_am_leader(&mut self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None {
                return false;
            }
            return *self.my_address.read().unwrap() == *leader_addr_mut.clone().unwrap();
        } else {
            // FIXME
            panic!("Not implemented");
        }
    }

    fn handle_coordinator_msg(&mut self, leader: SocketAddr) -> () {
        let (lock, cvar) = &*self.leader_condvar;
        let mut leader_found = lock.lock().unwrap();
        *leader_found = true;
        cvar.notify_all();

        if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
            *leader_addr_mut = Some(leader.to_string());
        }
        let mut leader_addr = (*self.leader_addr.read().unwrap()).clone();

        println!(
            "New leader found in address: {}",
            leader_addr.get_or_insert("??".to_string())
        );

        let (lock, _) = &*self.leader_down;
        *lock.lock().unwrap() = false;
        *self.running_bully.lock().unwrap() = false;
    }

    fn check_if_i_am_leader(&mut self, node_that_asked: String) -> () {
        if self.i_am_leader() {
            self.socket
                .send_to(COORDINATOR.to_string(), node_that_asked.clone())
                .unwrap();
            self.send_blockchain(node_that_asked.clone());
        }
    }

    fn send_blockchain(&mut self, from: String) {
        self.socket
            .send_to(BLOCKCHAIN.to_string(), from.clone())
            .unwrap();
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
                .send_to(data_to_send, from.clone().to_string())
                .unwrap();
        }
        self.socket.send_to(END.to_string(), from).unwrap();
    }

    fn recv_blockchain(&mut self) -> Blockchain {
        let mut blockchain = Blockchain::new();
        loop {
            let (_, from, msg) = self.socket.recv_from();
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
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        record
    }
}
