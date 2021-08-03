use crate::utils::messages::*;
use crate::utils::socket::Socket;
use crate::utils::logger::Logger;

use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::time::Duration;

const LEADER_DISCOVER_TIMEOUT_SECS: u64 = 2;

/// Responsible for discover who the leader addrs is 
/// and change the value from the leader
/// only the first time when the node is conected
pub struct LeaderDiscoverer {
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub my_address: Arc<RwLock<String>>,
    pub socket: Socket,
    pub other_nodes: Arc<Vec<String>>,
    pub logger: Arc<Logger>
}

impl LeaderDiscoverer {
    pub fn new(
        condvar: Arc<(Mutex<bool>, Condvar)>,
        leader_addr: Arc<RwLock<Option<String>>>,
        my_address: Arc<RwLock<String>>,
        socket: Socket,
        other_nodes: Arc<Vec<String>>,
        logger: Arc<Logger>
    ) -> Self {
        LeaderDiscoverer {
            condvar,
            leader_addr,
            my_address,
            socket,
            other_nodes,
            logger
        }
    }

    // Este algoritmo es el primero que corre cada nodo
    // para tratar de encontrar a otro lider
    // envia mensajes para tratar de encontrar a otro lider
    // si falla (timeout) entonces se setea a si mismo.
    pub fn run(&mut self) -> () {
        for node in &*self.other_nodes {
            self.socket
                .send_to(WHO_IS_LEADER.to_string(), node.clone())
                .unwrap();
        }

        let time = time::Instant::now();

        let (lock, cvar) = &*self.condvar;
        let mut leader_found = lock.lock().unwrap();

        // TODO: Review this: wait_timeout_while?
        loop {
            let result = cvar
                .wait_timeout(leader_found, Duration::from_millis(1000))
                .unwrap();
            let now = time::Instant::now();
            leader_found = result.0;
            if *leader_found == true {
                break;
            } else if now.duration_since(time).as_secs() >= LEADER_DISCOVER_TIMEOUT_SECS {
                self.logger.info(format!("TIMEOUT: Leader not found, I become leader"));
                if let Ok(mut leader_addr_mut) = self.leader_addr.write() {
                    *leader_addr_mut = Some((*self.my_address.read().unwrap()).clone());

                    for node in &*self.other_nodes {
                        self.socket
                            .send_to(COORDINATOR.to_string(), node.clone())
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
