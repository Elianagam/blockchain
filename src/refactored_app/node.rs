use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use std::thread;

use crate::blockchain::Blockchain;
use crate::messages::*;
use std::time::Duration;

pub struct Node {
    pub my_address: String,
    pub socket: UdpSocket,
    pub other_nodes: Vec<String>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub blockchain: Blockchain,
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
            my_address: my_address.clone(),
            socket: UdpSocket::bind(my_address.clone()).unwrap(),
            other_nodes: other_nodes,
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Blockchain::new()
        }
    }

    pub fn run(&mut self) -> () {
        println!("Running node on: {} ", self.socket.local_addr().unwrap().to_string());

        let pair = Arc::new((Mutex::new(false), Condvar::new()));

        for node in &self.other_nodes
        {
            self.socket.send_to(&encode_to_bytes(WHO_IS_LEADER), node).unwrap();
        }

        let leader_addr = self.leader_addr.clone();
        let pair_clone = pair.clone();

        thread::spawn(move || {
            wait_for_leader(pair_clone.clone(), leader_addr.clone());
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
                    self.leader_found(Arc::clone(&pair));
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
        println!("my address: {} " , self.my_address);
        return self.my_address == "127.0.0.1:8080";
        /*if let Ok(mut leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None 
            {
                return false;
            }
            println!("Leader addr: {:?}", *leader_addr_mut);
            return true; //self.my_address == *leader_addr_mut.clone().unwrap();
        }
        else {
            println!("not implemented");
            return false;
        }*/
    }

    fn leader_found(&self, pair: Arc<(Mutex<bool>, Condvar)>) -> () {
        let (lock, cvar) = &*pair;
        let mut leader_found = lock.lock().unwrap();
        *leader_found = true;
        cvar.notify_one();
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
        //println!("WHAT");
    }

}

fn wait_for_leader(pair: Arc<(Mutex<bool>, Condvar)>, leader_addr:Arc<RwLock<Option<String>>>)
{
    println!("Waiting for leader");
    let (lock, cvar) = &*pair;
    let mut leader_found = lock.lock().unwrap();
    // as long as the value inside the `Mutex<bool>` is `false`, we wait
    loop {
        println!("searching leader");
        let result = cvar.wait_timeout(leader_found, Duration::from_millis(5000)).unwrap();
        // 10 milliseconds have passed, or maybe the value changed!
        leader_found = result.0;
        if *leader_found == true {
            println!("Leader found");
            break
        }
        else
        {
            println!("Leader not found, I become leader");
            if let Ok(mut leader_addr_mut) = leader_addr.write()
            {
                println!("Setting leader addr to mine");
                *leader_addr_mut = Some("127.0.0.1:8080".to_string());
            }
            // Set leader addr!!!!!
            break;
        }
    }
}