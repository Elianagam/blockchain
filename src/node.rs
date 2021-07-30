use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use std::thread;
use std::time::Duration;
use std::time;
use crate::blockchain::blockchain::Blockchain;
use crate::utils::messages::*;

pub struct Node {
    pub my_address: Arc<RwLock<String>>,
    pub socket: UdpSocket,
    pub other_nodes: Arc<Vec<String>>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
    pub leader_found: bool,
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
            leader_found: false
        }
    }

    pub fn run(&mut self) -> () {
        println!("Running node on: {} ", self.socket.local_addr().unwrap().to_string());

        let pair = Arc::new((Mutex::new(false), Condvar::new()));

        for node in &*self.other_nodes
        {
            self.socket.send_to(&encode_to_bytes(WHO_IS_LEADER), node).unwrap();
        }

        let leader_addr = self.leader_addr.clone();
        let pair_clone = pair.clone();
        let my_addres_clone =  self.my_address.clone();
        let socket_clone= self.socket.try_clone().expect("msg");
        let other_nodes_clone = self.other_nodes.clone();

        thread::spawn(move || {
            wait_for_leader(pair_clone, leader_addr, my_addres_clone,
                socket_clone, other_nodes_clone);
        });


        // TODO: poner la logica en el thread de arriba, ir haciendo workers para los threads
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
                    self.leader_found(Arc::clone(&pair), from);
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
    
    fn i_know_the_leader(&self) -> bool {
        if let Ok(leader_addr_mut) = self.leader_addr.read() {
            return !leader_addr_mut.is_none();
        }
        else{
            println!("not implemented");
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
        //return self.my_address == "127.0.0.1:8080";
        if let Ok(mut leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None 
            {
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

    fn leader_found(&self, pair: Arc<(Mutex<bool>, Condvar)>, leader: SocketAddr) -> () {

        let (lock, cvar) = &*pair;
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
            print!("Sending to {} that I am leader", node_that_asked.to_string());
            self.socket.send_to(&encode_to_bytes(I_AM_LEADER), node_that_asked).unwrap();
        }        
        else {
            println!("I am not leader");
        }
    }

}

fn wait_for_leader(pair: Arc<(Mutex<bool>, Condvar)>, leader_addr:Arc<RwLock<Option<String>>>, 
                    my_address: Arc<RwLock<String>>, socket: UdpSocket, other_nodes: Arc<Vec<String>>)
{
    println!("Waiting for leader");

    let time = time::Instant::now();

    let (lock, cvar) = &*pair;
    let mut leader_found = lock.lock().unwrap();

    loop {
        let result = cvar.wait_timeout(leader_found, Duration::from_millis(1000)).unwrap();
        println!("searching leader");
        let now = time::Instant::now();
        leader_found = result.0;
        println!("Duration: {}", now.duration_since(time).as_secs());
        if *leader_found == true {
            println!("Leader found");
            break
        }
        else if now.duration_since(time).as_secs() >= 10
        {
            println!("TIMEOUT: Leader not found, I become leader");
            if let Ok(mut leader_addr_mut) = leader_addr.write()
            {
                println!("Setting leader addr to mine");
                *leader_addr_mut = Some((*my_address.read().unwrap()).clone());
                
                for node in &*other_nodes
                {
                    socket.send_to(&encode_to_bytes(I_AM_LEADER), node).unwrap();
                }
                
            }
            break;
        }
    }
}