use crate::encoder::encode_to_bytes;
use crate::utils::messages::*;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::time::Duration;

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

    pub fn run(&mut self) -> () {
        let time = time::Instant::now();

        let (lock, cvar) = &*self.condvar;
        let mut leader_found = lock.lock().unwrap();

        loop {
            let result = cvar
                .wait_timeout(leader_found, Duration::from_millis(1000))
                .unwrap();
            let now = time::Instant::now();
            leader_found = result.0;
            if *leader_found == true {
                break;
            } else if now.duration_since(time).as_secs() >= 1 {
                println!("TIMEOUT: Leader not found, I become leader\n");
                if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
                    *leader_addr_mut = Some((*self.my_address.read().unwrap()).clone());

                    for node in &*self.other_nodes {
                        self.socket
                            .send_to(&encode_to_bytes(I_AM_LEADER), node)
                            .unwrap();
                    }
                }
                break;
            }
        }
    }
}
