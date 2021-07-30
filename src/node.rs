use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use std::thread;
use std_semaphore::Semaphore;
use std::time::Duration;
use crate::blockchain::blockchain::Blockchain;
use crate::leader_discoverer::LeaderDiscoverer;
use crate::utils::messages::*;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: UdpSocket,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub mutex: Arc<Semaphore>,
    pub pending_acquires: Vec<String>,
    pub node_id_with_mutex: Arc<RwLock<Option<SocketAddr>>>
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
            leader_condvar: Arc::new((Mutex::new(false), Condvar::new())),
            mutex: Arc::new(Semaphore::new(1)),
            pending_acquires: vec![],
            node_id_with_mutex: Arc::new(RwLock::new(None))
        }
    }

    pub fn run(&mut self) -> () {
        println!("Running node on: {} ", self.socket.local_addr().unwrap().to_string());

        self.discover_leader();

        let socket_clone = self.socket.try_clone().unwrap();
        let leader_addr_clone = self.leader_addr.clone();
        let my_address_clone = self.my_address.clone();

        thread::spawn(move || {
            let leader_addr = (leader_addr_clone.read().unwrap()).clone();
            while leader_addr.is_none()
            {
                // DO nothing
            }
            
            if *my_address_clone.read().unwrap() != (*leader_addr_clone.read().unwrap()).clone().unwrap()
            {
                socket_clone.send_to(&encode_to_bytes(ACQUIRE_MSG), leader_addr.clone().unwrap()).unwrap();
                println!("Sent acquire");

                thread::sleep(Duration::from_secs(10));

                socket_clone.send_to(&encode_to_bytes(RELEASE_MSG), leader_addr.clone().unwrap()).unwrap();
                println!("Sent release");
            }
            
        });

        loop {
            let (msg, from)= self.read_from();
            match msg.as_str() {
                WHO_IS_LEADER => {
                    println!("A node is asking who the leader is");
                    if self.i_know_the_leader() 
                    {
                        println!("I know the leader");
                        self.check_if_i_am_leader(from.to_string());
                    }
                    else{
                        println!("I don't know the leader");
                    }
                }
                I_AM_LEADER => {
                    println!("Someone said he is leader");
                    self.leader_found(from);
                }
                ACQUIRE_MSG => {
                    if self.i_am_leader() {
                        let node_id_with_mutex_clone = self.node_id_with_mutex.clone();
                        self.node_attempting_to_acquire_mutex(from, self.mutex.clone(), node_id_with_mutex_clone);
                    }
                }
                RELEASE_MSG => {
                    if self.i_am_leader() {
                        self.node_releasing_mutex(from);
                    }
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

    fn discover_leader(&self) -> () {
        for node in &*self.other_nodes
        {
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
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None 
            {
                return false;
            }
            return *self.my_address.read().unwrap() == *leader_addr_mut.clone().unwrap();
        }
        else {
            println!("Leader is being changed");
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
        if self.i_am_leader()
        {
            println!("Sending to {} that I am leader", node_that_asked.to_string());
            self.socket.send_to(&encode_to_bytes(I_AM_LEADER), node_that_asked).unwrap();
        }        
        else {
            println!("I am not leader");
        }
    }


    // TODO: Cambiar esto para tener un thread 


    // Esto bloquea el thread de recepción de mensajes => deberia ir en otro thread
    fn node_attempting_to_acquire_mutex(&mut self, node: SocketAddr, 
                                        mutex: Arc<Semaphore>, node_id_with_mutex: Arc<RwLock<Option<SocketAddr>>>) {
        if node.to_string() == *self.my_address.read().unwrap() {
            return;
        }
        println!("Received acquire form: {}", node);
        //self.pending_acquires.push(node.to_string());
        println!("{} waiting for mutex to be released", node);
        thread::spawn(move || {
            mutex.acquire();
            if let Ok(mut node_id) = node_id_with_mutex.write() {
                *node_id = Some(node);
            }
            println!("Mutex acquired by {}", node.to_string());
        });        
    }

    fn node_releasing_mutex(&mut self, node: SocketAddr) {
        if node.to_string() == *self.my_address.read().unwrap() {
            return;
        }
        if node != (*self.node_id_with_mutex.read().unwrap()).unwrap() {
            return;
        }
        self.mutex.release();
        if let Ok(mut node_id) = self.node_id_with_mutex.write() {
            *node_id = None;
        }
        println!("{} released mutex", node.to_string());
    }
}