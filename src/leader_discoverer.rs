use crate::encoder::encode_to_bytes;
use crate::utils::messages::*;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::time::Duration;

const LEADER_DISCOVER_TIMEOUT_SECS: u64 = 2;

pub struct LeaderDiscoverer {
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub my_address: Arc<RwLock<String>>,
    pub socket: UdpSocket,
    pub other_nodes: Arc<Vec<String>>,
}

impl LeaderDiscoverer {
    pub fn new(
        condvar: Arc<(Mutex<bool>, Condvar)>,
        leader_addr: Arc<RwLock<Option<String>>>,
        my_address: Arc<RwLock<String>>,
        socket: UdpSocket,
        other_nodes: Arc<Vec<String>>,
    ) -> Self {
        LeaderDiscoverer {
            condvar,
            leader_addr,
            my_address,
            socket,
            other_nodes,
        }
    }

    // Este algoritmo es el primero que corre cada nodo
    // para tratar de encontrar a otro lider
    // envia mensajes para tratar de encontrar a otro lider
    // si falla (timeout) entonces se setea a si mismo.
    pub fn run(&mut self) -> () {
        for node in &*self.other_nodes {
            self.socket
                .send_to(&encode_to_bytes(WHO_IS_LEADER), node)
                .unwrap();
        }

        println!("Waiting for leader");
        let time = time::Instant::now();

        let (lock, cvar) = &*self.condvar;
        let mut leader_found = lock.lock().unwrap();

        loop {
            let result = cvar
                .wait_timeout(leader_found, Duration::from_millis(1000))
                .unwrap();
            println!("searching leader");
            let now = time::Instant::now();
            leader_found = result.0;
            if *leader_found == true {
                println!("Leader found");
                break;
            } else if now.duration_since(time).as_secs() >= LEADER_DISCOVER_TIMEOUT_SECS {
                println!("TIMEOUT: Leader not found, I become leader");
                if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
                    println!("Setting leader addr to mine");
                    *leader_addr_mut = Some((*self.my_address.read().unwrap()).clone());

                    for node in &*self.other_nodes {
                        self.socket
                            .send_to(&encode_to_bytes(I_AM_LEADER), node)
                            .unwrap();
                    }
                }
                *leader_found = true;
                cvar.notify_all();
                break;
            }
        }
    }
}
