use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use std::thread;
use crate::blockchain::blockchain::Blockchain;
use crate::leader_discoverer::LeaderDiscoverer;
use crate::utils::messages::*;
use crate::stdin_reader::StdinReader;
use std::io::{self, BufRead};

fn read_stdin() -> String {
    println!("Write a student note:");
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    let line = iterator.next().unwrap().unwrap();

    let student_data: Vec<&str> = line.split(",").collect();
    if (student_data.len() == 1 && (student_data[0] == "close")) || student_data.len() == 2 {
        return line.to_string();
    } else {
        println!("Unsupported data format, usage: id, qualification")
    }
    return String::new(); 
}

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: UdpSocket,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>
}

impl Node {
    pub fn new(port_number: &str) -> Self {
        let mut other_nodes = vec!["127.0.0.1:8080".to_string(), 
                                          "127.0.0.1:8081".to_string(),
                                          "127.0.0.1:8082".to_string(),
                                          "127.0.0.1:8083".to_string(),
                                          "127.0.0.1:8084".to_string()];

        let mut my_address: String = "127.0.0.1:".to_owned();
        my_address.push_str(port_number);
        other_nodes.retain(|x| x != &my_address);
    
        Node {
            my_address: Arc::new(RwLock::new(my_address.clone())),
            socket: UdpSocket::bind(my_address.clone()).unwrap(),
            other_nodes: Arc::new(other_nodes),
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Blockchain::new(),
            leader_condvar: Arc::new((Mutex::new(false), Condvar::new()))
        }
    }

    pub fn run(&mut self) -> () {
        self.discover_leader();

        loop {
            let (msg, from)= self.read_from();
            match msg.as_str() {
                WHO_IS_LEADER => {
                    println!("A node is asking who the leader is");
                    if self.i_know_the_leader() {
                        self.check_if_i_am_leader(from.to_string());
                    }
                    let clone_addr = (*self.leader_addr.read().unwrap()).clone();
                    let clone_socket = self.socket.try_clone().unwrap();
                    thread::spawn(move || {
                        let reader = StdinReader::new(clone_socket, clone_addr);
                        reader.run();
                    });
                }
                I_AM_LEADER => {
                    self.leader_found(from);
                    let clone_addr = (*self.leader_addr.read().unwrap()).clone();
                    let clone_socket = self.socket.try_clone().unwrap();
                    thread::spawn(move || {
                        let reader = StdinReader::new(clone_socket, clone_addr);
                        reader.run();
                    });
                    // send blockchain
                }
                CLOSE => {
                    break;
                }
                msg => {
                    println!("\nReceived {}", msg);
                }
            }
        }

        println!("Desconectando...");
        
    }

    fn discover_leader(&self) -> () {
        for node in &*self.other_nodes {
            self.socket.send_to(&encode_to_bytes(WHO_IS_LEADER), node).unwrap();
        }

        let mut leader_discoverer = LeaderDiscoverer::new(
            self.leader_condvar.clone(),
            self.leader_addr.clone(),
            self.my_address.clone(),
            self.socket.try_clone().expect("Error cloning socket"),
            self.other_nodes.clone()
        );
        
        thread::spawn(move || {
            leader_discoverer.run();
        });
    }
    
    fn i_know_the_leader(&self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        }
        else{
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
        println!("my address: {} " , self.my_address.read().unwrap());

        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None {
                return false;
            }
            println!("Leader addr: {:?}", *leader_addr_mut);
            return *self.my_address.read().unwrap() == *leader_addr_mut.clone().unwrap();
        }
        else {
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
        println!("La direccion del leader es: {}", leader_addr.get_or_insert("No address".to_string()));
    }

    fn check_if_i_am_leader(&self, node_that_asked: String) -> () {
        println!("Checking if I'm leader");
        if self.i_am_leader() {
            print!("Sending to {} that I am leader", node_that_asked.to_string());
            self.socket.send_to(&encode_to_bytes(I_AM_LEADER), node_that_asked).unwrap();
        }        
        else {
            println!("I am not leader");
        }
    }
}