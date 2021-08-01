use crate::blockchain::blockchain::Blockchain;
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use crate::leader_discoverer::LeaderDiscoverer;
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

        thread::spawn(move || {
            let mut leader_addr = (leader_addr_clone.read().unwrap()).clone();
            // FIXME: condvar
            while leader_addr.is_none() {
                leader_addr = (leader_addr_clone.read().unwrap()).clone();
                // DO nothing
            }
            let reader = StdinReader::new(clone_socket, leader_addr);
            reader.run();
        });

        loop {
            let (msg, from)= self.read_from();
            println!("MSG: {} - FROM: {}",msg, from);
            match msg.as_str() {
                WHO_IS_LEADER => {
                    println!("A node is asking who the leader is");
                    if self.i_know_the_leader() {
                        println!("I know the leader");
                        self.check_if_i_am_leader(from.to_string());
                    } else {
                        println!("I don't know the leader");
                    }
                }
                I_AM_LEADER => {
                    self.leader_found(from);
                }
                CLOSE => {
                    let mut clone_addr = (*self.leader_addr.read().unwrap()).clone();
                    let leader_addr = format!("{}", clone_addr.get_or_insert("Error".to_string()));
                    if format!("{}", from) == leader_addr {
                        // Me cierro a mi mismo
                        break;
                    } else {
                        // Si no es el leader le mando el mensaje para que se cierre 
                        self.socket.send_to(&encode_to_bytes(&msg), from).unwrap();
                    }
                }
                BLOCKCHAIN => {
                    println!("No soy el leader, recibir blockchain...");
                    self.blockchain = self.recv_blockchain();
                    println!("{}", self.blockchain);
                }
                msg => {
                    let mut clone_addr = (*self.leader_addr.read().unwrap()).clone();
                    let leader_addr = format!("{}", clone_addr.get_or_insert("Error".to_string()));

                    let record = self.create_record(msg, from);
                    let tmp = record.clone();
                    let mut block = Block::new(self.blockchain.get_last_block_hash());
                    block.add_record(record);
                    if let Err(err) = self.blockchain.append_block(block) {
                        println!("Error: {}", err);
                    } 
                    if self.i_am_leader() {
                        // Si el mensaje viene del leader, lo propago a todos
                        println!("IF - RECV: {} - FROM LEADER?: {} - {}", msg, tmp.from, leader_addr);
                        for node in &*self.other_nodes {
                            self.socket.send_to(&encode_to_bytes(msg), node).unwrap();
                        }
                    } else {
                        println!("ELSE - RECV: {} - FROM LEADER?: {} - {}", msg, tmp.from, leader_addr);
                    } 
                    println!("{}", self.blockchain);

                }
            }
        }

        println!("Desconectando...");
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

    fn discover_leader(&self) -> () {
        for node in &*self.other_nodes {
            self.socket
                .send_to(&encode_to_bytes(WHO_IS_LEADER), node)
                .unwrap();
        }

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
