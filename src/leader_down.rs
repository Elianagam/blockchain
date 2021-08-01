use crate::utils::messages::*;
use crate::encoder::encode_to_bytes;
use std::net::UdpSocket;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time;
use std::time::Duration;

const LEADER_DISCOVER_TIMEOUT_SECS: u64 = 2;

pub struct LeaderDown {
    pub condvar: Arc<(Mutex<bool>, Condvar)>,
    pub leader_addr: Arc<RwLock<Option<String>>>,
    pub socket: UdpSocket,
    other_nodes: Arc<Vec<String>>,
}

impl LeaderDown {
    pub fn new(
        condvar: Arc<(Mutex<bool>, Condvar)>,
        leader_addr: Arc<RwLock<Option<String>>>,
        socket: UdpSocket,
        other_nodes: Arc<Vec<String>>,
    ) -> Self {
        LeaderDown {
            condvar,
            leader_addr,
            socket,
            other_nodes
        }
    }

    // Este algoritmo es el primero que corre cada nodo
    // para tratar de encontrar a otro lider
    // envia mensajes para tratar de encontrar a otro lider
    // si falla (timeout) entonces se setea a si mismo.
    pub fn run(&mut self) -> () {
        let time = time::Instant::now();
        let (lock, cvar) = &*self.condvar;
        let mut leader_down = lock.lock().unwrap();

        loop {
            let result = cvar
                .wait_timeout(leader_down, Duration::from_millis(1000))
                .unwrap();
            let now = time::Instant::now();
            leader_down = result.0;
            if *leader_down == true {
                println!("Leader is not down");
                break;
            } else if now.duration_since(time).as_secs() >= LEADER_DISCOVER_TIMEOUT_SECS {
                println!("TIMEOUT: Leader is down");
                
                for node in &*self.other_nodes {
                    self.socket
                        .send_to(&encode_to_bytes(LEADER_IS_DOWN), node)
                        .unwrap();
                }
                
                *leader_down = true;
                cvar.notify_all();
                break;
            }
        }
    }
}
