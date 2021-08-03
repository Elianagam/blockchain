use crate::blockchain::block::Block;
use crate::blockchain::blockchain::Blockchain;
use crate::blockchain::record::{Record, RecordData};
use crate::leader_discoverer::LeaderDiscoverer;
use crate::leader_down_handler::LeaderDownHandler;
use crate::stdin_reader::StdinReader;
use crate::utils::messages::*;
use crate::utils::socket::Socket;

use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};
use std_semaphore::Semaphore;

const MAX_NODES: u32 = 50;
const MUTEX_HOLD_TIMEOUT_SECS: u64 = 30;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: Socket,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub lock_acquired: Arc<(Mutex<bool>, Condvar)>,
    pub mutex: Arc<Semaphore>,
    pub node_id_with_mutex: Arc<RwLock<Option<SocketAddr>>>,
    pub election_condvar: Arc<(Mutex<Option<String>>, Condvar)>,

    // Cantidad de nodos que tomaron el mutex pero que no lo liberaron
    // debería ser en el peor de los casos 1 (si no hacemos un panic).
    pub not_released_nodes: Arc<RwLock<i32>>,

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
            socket: Socket::new(socket),
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Arc::new(RwLock::new(Blockchain::new())),
            leader_condvar: Arc::new((Mutex::new(false), Condvar::new())),
            lock_acquired: Arc::new((Mutex::new(false), Condvar::new())),
            mutex: Arc::new(Semaphore::new(1)),
            node_id_with_mutex: Arc::new(RwLock::new(None)),
            election_condvar: Arc::new((Mutex::new(None), Condvar::new())),
            alive: Arc::new(RwLock::new(true)),
            not_released_nodes: Arc::new(RwLock::new(0)),
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
                ACQUIRE_MSG => self.handle_acquire_msg(from, self.mutex.clone(), self.node_id_with_mutex.clone()),
                RELEASE_MSG => self.handle_release_msg(from),
                LOCK_ACQUIRED => self.handle_lock_acquired(),
                WHO_IS_LEADER => self.handle_who_is_leader(from),
                COORDINATOR => self.handle_coordinator_msg(from),
                BLOCKCHAIN => self.handle_blockchain_msg(),
                OK => self.handle_ok_msg(from),
                ELECTION => self.handle_election_msg(from),
                ACK_MSG => self.handle_ack_msg(),
                NOOP_MSG => {},
                msg => self.handle_msg(msg, from),
            }
        }
    }

    fn handle_who_is_leader(&mut self, from: SocketAddr) {
        if self.i_know_the_leader() {
            self.check_if_i_am_leader(from.to_string());
        }
    }

    fn handle_blockchain_msg(&mut self) {
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
        if let Ok(mut blockchain_mut) = self.blockchain.write() {
            *blockchain_mut = blockchain;
        }
    }

    fn handle_ok_msg(&mut self, from: SocketAddr) {
        // Basicamente cada vez que recibamos un mensaje le hacemos un notify
        // a la otra convar y seteamos la IP del que recibimos.
        let (lock, cvar) = &*self.election_condvar;
        *lock.lock().unwrap() = Some(from.to_string());
        cvar.notify_all();
    }

    fn handle_election_msg(&mut self, from: SocketAddr) {
        self.socket
            .send_to(OK.to_string(), from.to_string())
            .unwrap();

        let (lock, cvar) = &*self.leader_down;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    fn handle_ack_msg(&mut self) {
        let (lock, cv) = &*self.msg_ack_cv;
        *lock.lock().unwrap() = true;
        cv.notify_all();
    }

    fn handle_msg(&mut self, msg: &str, from: SocketAddr) {
        let record = self.create_record(msg, from);

        if let Ok(mut blockchain_mut) = self.blockchain.write() {
            let mut block = Block::new(blockchain_mut.get_last_block_hash());
            block.add_record(record);
            match blockchain_mut.append_block(block) {
                Err(_) => println!("Error"),
                Ok(_) => {}
            };
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
    }

    fn stdin_reader(&mut self) {
        let mut reader = StdinReader::new(
            self.leader_condvar.clone(),
            self.socket.try_clone(),
            self.leader_addr.clone(),
            self.alive.clone(),
            self.msg_ack_cv.clone(),
            self.leader_down.clone(),
            self.lock_acquired.clone(),
            self.blockchain.clone()
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

    fn handle_lock_acquired(&self) -> () {
        let (lock, cvar) = &*self.lock_acquired;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    fn handle_coordinator_msg(&mut self, leader: SocketAddr) -> () {
        let (lock, cvar) = &*self.leader_condvar;
        *lock.lock().unwrap() = true;
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

    fn handle_acquire_msg(
        &mut self,
        node: SocketAddr,
        mutex: Arc<Semaphore>,
        node_id_with_mutex: Arc<RwLock<Option<SocketAddr>>>,
    ) {
        let mut socket_clone = self.socket.try_clone();
        let not_released_nodes_clone = self.not_released_nodes.clone();

        thread::spawn(move || {
            mutex.acquire();

            if let Ok(mut not_released_nodes) = not_released_nodes_clone.write() {
                *not_released_nodes = *not_released_nodes + 1;
            }

            if let Ok(mut node_id) = node_id_with_mutex.write() {
                *node_id = Some(node);
            }
            socket_clone
                .send_to(LOCK_ACQUIRED.to_string(), node.to_string())
                .unwrap();


            // Si paso un timeout sin recibir RELEASE, hago un release
            thread::sleep(Duration::from_secs(MUTEX_HOLD_TIMEOUT_SECS));

            if let Ok(mut not_released_nodes) = not_released_nodes_clone.write() {
                if *not_released_nodes > 1 { panic!("not_released_nodes > 1") }

                if *not_released_nodes == 1 {
                    mutex.release();
                    *not_released_nodes = 0;
                    println!(
                        "Mutex released because node {} was disconnected",
                        node.to_string()
                    );
                }
            }
        });
    }

    fn handle_release_msg(&mut self, node: SocketAddr) {
        // Only if the node that had the mutex sent the release
        if node != (*self.node_id_with_mutex.read().unwrap()).unwrap() {
            return;
        }

        if let Ok(mut not_released_nodes) = self.not_released_nodes.write() {
            if *not_released_nodes == 1 {
                self.mutex.release();
                *not_released_nodes = 0;
            } else if *not_released_nodes > 1 {
                panic!("not_released_nodes > 1")
            }
        }

        if let Ok(mut node_id) = self.node_id_with_mutex.write() {
            *node_id = None;
        }
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

        if let Ok(blockchain_mut) = self.blockchain.read() {
            for b in blockchain_mut.get_blocks() {
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
        }
        self.socket.send_to(END.to_string(), from).unwrap();
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
            Duration::from_millis(student_data[2].parse::<u64>().unwrap()),
        );
        record
    }
}
