use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, Condvar, RwLock};
use crate::encoder::{decode_from_bytes, encode_to_bytes};
use std::thread;

use crate::blockchain::Blockchain;
use crate::messages::*;
use std::time::Duration;
use std::time;

pub struct Node {
    pub my_address: String,
    pub socket: UdpSocket,
    pub other_nodes: Vec<String>,
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
            my_address: my_address.clone(),
            socket: UdpSocket::bind(my_address.clone()).unwrap(),
            other_nodes: other_nodes,
            leader_addr: Arc::new(RwLock::new(None)),
            blockchain: Blockchain::new(),
            leader_found: false
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

        let socket_clone= self.socket.try_clone().expect("msg");

        //TODO: meter toda la logica de receive dentro de este thread
        thread::spawn(move || {
            loop {
                let mut buf = [0; 128];
                let (_, from) = socket_clone.recv_from(&mut buf).unwrap();
                let msg = decode_from_bytes(buf.to_vec());
                match msg.as_str() {
                    WHO_IS_LEADER => {
                        println!("A node is asking who the leader is");
                        
                    }
                    I_AM_LEADER => {
                        println!("Someone said he is leader");
                        
                    }
                    CLOSE => {
                        break;
                    }
                    msg => {
                        println!("Received {}", msg);
                    }
                }
            }
                
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
        //return self.my_address == "127.0.0.1:8080";
        if let Ok(mut leader_addr_mut) = self.leader_addr.read() {
            if *leader_addr_mut == None 
            {
                return false;
            }
            println!("Leader addr: {:?}", *leader_addr_mut);
            return self.my_address == *leader_addr_mut.clone().unwrap();
        }
        else {
            // FIXME
            println!("not implemented");
            return false;
        }
    }

    fn leader_found(&self, pair: Arc<(Mutex<bool>, Condvar)>) -> () {

        let (lock, cvar) = &*pair;
        let mut leader_found = lock.lock().unwrap();
        *leader_found = true;
        cvar.notify_all();
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

// FIXME: Si se conectan 2 nodos casi al mismo tiempo, ambos se colocan como lideres porque este thread
// bloquea al de recepcion de mensajes y no llega a recibir el mensaje de que hay un lider ni el
// otro a recibir el mensaje de que alguien esta pidiendo por un lider

//SOLUCION!!:: ABRIR UN THREAD PARA LAS RESPONSES
fn wait_for_leader(pair: Arc<(Mutex<bool>, Condvar)>, leader_addr:Arc<RwLock<Option<String>>>)
{
    println!("Waiting for leader");

    let time = time::Instant::now();

    let (lock, cvar) = &*pair;
    let mut leader_found = lock.lock().unwrap();
    // as long as the value inside the `Mutex<bool>` is `false`, we wait
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
                *leader_addr_mut = Some("127.0.0.1:8080".to_string());
            }
            break;
        }
    }
}