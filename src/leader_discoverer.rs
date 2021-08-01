use crate::utils::messages::*;
use crate::utils::socket_with_timeout::SocketWithTimeout;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::time::Duration;

pub struct LeaderDiscoverer {
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub my_address: Arc<RwLock<String>>,
    pub socket: SocketWithTimeout,
    pub other_nodes: Arc<Vec<String>>,
}

impl LeaderDiscoverer {
    pub fn new(
        condvar: Arc<(Mutex<bool>, Condvar)>,
        leader_addr: Arc<RwLock<Option<String>>>,
        my_address: Arc<RwLock<String>>,
        socket: SocketWithTimeout,
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

    pub fn run(&mut self) -> () {
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
            println!("Duration: {}", now.duration_since(time).as_secs());
            if *leader_found == true {
                println!("Leader found");
                break;
            } else if now.duration_since(time).as_secs() >= 10 {
                println!("TIMEOUT: Leader not found, I become leader");
                if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
                    println!("Setting leader addr to mine");
                    *leader_addr_mut = Some((*self.my_address.read().unwrap()).clone());

                    for node in &*self.other_nodes {
                        self.socket
                            .send_to(COORDINATOR.to_string(), node.clone())
                            .unwrap();
                    }
                }
                break;
            }
        }
    }
}
