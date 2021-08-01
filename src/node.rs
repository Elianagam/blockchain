use crate::blockchain::blockchain::Blockchain;
use crate::stdin_reader::StdinReader;
use crate::blockchain::block::Block;
use crate::utils::messages::*;
use crate::leader_discoverer::LeaderDiscoverer;
use crate::blockchain::record::{RecordData, Record};
use crate::utils::socket_with_timeout::SocketWithTimeout;

use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::net::{UdpSocket, SocketAddr};
use std::time::{Duration, SystemTime};
use std::thread;

const MAX_NODES: u32 = 50;
const ELECTION_TIMEOUT_SECS: u64 = 1;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: SocketWithTimeout,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub election_condvar: Arc<(Mutex<Option<String>>, Condvar)>,
    pub alive: Arc<RwLock<bool>>
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

fn get_port_from_addr(addr: String) -> u32 {
    addr.split(":").collect::<Vec<&str>>()[1].parse::<u32>().unwrap()
}

fn find_upper_sockets(my_address: &String) -> Vec<String> {
    let mut upper_nodes = vec!();

    for n_addr in build_addr_list(&my_address) {
        if get_port_from_addr(my_address.clone()) < get_port_from_addr(n_addr.clone()) {
            upper_nodes.push(n_addr);
        }
    }
    upper_nodes
}

fn run_bully_algorithm(my_address: String, mut socket: SocketWithTimeout, election_condvar: Arc<(Mutex<Option<String>>, Condvar)>) {
    println!("Running bully algorithm.");

    for node in find_upper_sockets(&my_address) {
        socket.send_to(ELECTION.to_string(), node).unwrap();
    }
    let (lock, cvar) = &*election_condvar;

    let guard = lock.lock().unwrap();
    let timeout = Duration::from_secs(ELECTION_TIMEOUT_SECS);

    let result = cvar.wait_timeout(guard, timeout).unwrap();

    if (*result.0).is_none() {
        let mut addr_list = build_addr_list(&my_address);
        // FIXME. Agregamos nuestra direccion a la lista 
        // para poder setearnos en nuestro estado interno
        // que somos el coordinador.
        addr_list.push(my_address);

        for n_addr in addr_list {
            socket.send_to(COORDINATOR.to_string(), n_addr).unwrap();
        }

    }
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
            other_nodes,
            alive: Arc::new(RwLock::new(true)),
        }
    }

    pub fn run(&mut self) -> () {
        println!(
            "Running node on: {} ",
            self.socket.local_addr().to_string()
        );

        self.discover_leader();

        let clone_socket = self.socket.try_clone();
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

            println!("Leader found: {:?} ", leader_addr);
            let mut reader = StdinReader::new(clone_socket, leader_addr, alive_clone);
            reader.run();
        });

        let me = self.my_address.read().unwrap().clone();
        let socket = self.socket.try_clone();
        let cv = self.election_condvar.clone();

        while *self.alive.read().unwrap() {
            let (_, from, msg) = self.socket.recv_from();
            match msg.as_str() {
                WHO_IS_LEADER => {
                    if self.i_know_the_leader() {
                        self.check_if_i_am_leader(from.to_string());
                    }
                }
                COORDINATOR => {
                    self.leader_found(from);
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
                    let me = self.my_address.read().unwrap().clone();
                    let mut socket = self.socket.try_clone();
                    let cv = self.election_condvar.clone();
                    std::thread::spawn(move || {
                        socket.send_to(OK.to_string(), from.to_string()).unwrap();
                        run_bully_algorithm(me, socket, cv);
                    });
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
                            self.socket.send_to(msg.to_string(), node.clone()).unwrap();
                        }
                    }
                    println!("{}", self.blockchain);

                }
            }
        }

        println!("Desconectando...");
    }

    fn send_blockchain(&mut self, from: String) {
        self.socket.send_to(BLOCKCHAIN.to_string(), from.clone()).unwrap();
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

    fn i_know_the_leader(&mut self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        } else {
            println!("Leader is being changed");
            return false;
        }
    }

    fn i_am_leader(&mut self) -> bool {
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
    fn leader_found(&mut self, leader: SocketAddr) -> () {
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

    fn check_if_i_am_leader(&mut self, node_that_asked: String) -> () {
        println!("Checking if I'm leader");
        if self.i_am_leader() {
            print!(
                "Sending to {} that I am leader",
                node_that_asked.to_string()
            );
            self.socket
                .send_to(COORDINATOR.to_string(), node_that_asked.clone())
                .unwrap();
            self.send_blockchain(node_that_asked.clone());
        } else {
            println!("I am not leader");
        }
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
