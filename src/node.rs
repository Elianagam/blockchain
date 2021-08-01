use crate::blockchain::blockchain::Blockchain;
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use crate::leader_discoverer::LeaderDiscoverer;
use crate::utils::socket_with_timeout::SocketWithTimeout;
use crate::utils::messages::*;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

const MAX_NODES: u32 = 50;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: SocketWithTimeout,
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

fn get_port_from_addr(addr: String) -> u32 {
    addr.split(":").collect::<Vec<&str>>()[1].parse::<u32>().unwrap()
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
            other_nodes,
        }
    }

    pub fn run(&mut self) -> () {
        println!(
            "Running node on: {} ",
            self.socket.local_addr().to_string()
        );

        self.discover_leader();

        loop {
            let (_, from, msg) = self.socket.recv_from();
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
                    println!("Someone said he is leader");
                    self.leader_found(from);
                }
                CLOSE => {
                    break;
                }
                msg => {
                    println!("Received {}", msg);
                }
            }
        }

        println!("Desconectando...");
    }

    fn find_upper_sockets(&mut self) -> Vec<String> {
        let me = self.my_address.read().unwrap().clone();

        let mut upper_nodes = vec!();

        for n_addr in build_addr_list(&me) {
            if get_port_from_addr(me.clone()) < get_port_from_addr(n_addr.clone()) {
                upper_nodes.push(n_addr);
            }
        }
        upper_nodes
    }

    fn run_bully_algorithm(&mut self) {
        println!("[NODE] Running bully algorithm.");

        for node in self.find_upper_sockets() {
            self.socket.send_to(ELECTION.to_string(), node);
        }
    }

    fn discover_leader(&mut self) -> () {
        for node in &*self.other_nodes {
            self.socket 
                .send_to(WHO_IS_LEADER.to_string(), node.clone());
        }

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

    fn i_know_the_leader(&self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        } else {
            println!("Leader is being changed");
            return false;
        }
    }

    fn i_am_leader(&self) -> bool {
        println!("my address: {} ", self.my_address.read().unwrap());

        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None {
                return false;
            }
            println!("Leader addr: {:?}", *leader_addr_mut);
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

    fn check_if_i_am_leader(&mut self, node_that_asked: String) -> () {
        println!("Checking if I'm leader");
        if self.i_am_leader() {
            print!(
                "Sending to {} that I am leader",
                node_that_asked.to_string()
            );
            self.socket
                .send_to(I_AM_LEADER.to_string(), node_that_asked)
                .unwrap();
        } else {
            println!("I am not leader");
        }
    }
}
