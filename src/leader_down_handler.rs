use crate::utils::messages::*;
use crate::utils::socket_with_timeout::SocketWithTimeout;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;

const MAX_NODES: u32 = 50;
const ELECTION_TIMEOUT_SECS: u64 = 1;
pub struct LeaderDownHandler {
    pub my_address: Arc<RwLock<String>>,
    pub socket: SocketWithTimeout,
    pub election_condvar: Arc<(Mutex<Option<String>>, Condvar)>,
    pub leader_down: Arc<(Mutex<bool>, Condvar)>,
    pub running_bully: Arc<Mutex<bool>>,
}

impl LeaderDownHandler {
    pub fn new(
        my_address: Arc<RwLock<String>>,
        socket: SocketWithTimeout,
        election_condvar: Arc<(Mutex<Option<String>>, Condvar)>,
        leader_down: Arc<(Mutex<bool>, Condvar)>,
        running_bully: Arc<Mutex<bool>>,
    ) -> Self {
        LeaderDownHandler {
            my_address,
            socket,
            election_condvar,
            leader_down,
            running_bully,
        }
    }

    pub fn run(&mut self) -> () {
        loop {
            let (lock, cv) = &*self.leader_down;

            {
                let mut leader_down = lock.lock().unwrap();
                // *guard: el lider murio
                while !*leader_down {
                    leader_down = cv.wait(leader_down).unwrap();
                }
            }

            if !*self.running_bully.lock().unwrap() {
                *self.running_bully.lock().unwrap() = true;
                self.run_bully_algorithm();
            }
        }
    }

    fn run_bully_algorithm(&mut self) {
        println!("<> Running bully algorithm. <>");

        for node in self.find_upper_sockets() {
            self.socket.send_to(ELECTION.to_string(), node).unwrap();
        }
        let (lock, cvar) = &*self.election_condvar;
        let current_value;

        let timeout = Duration::from_secs(ELECTION_TIMEOUT_SECS);

        {
            let guard = lock.lock().unwrap();
            let result = cvar.wait_timeout(guard, timeout).unwrap();

            current_value = (*result.0).clone();
        }

        if current_value.is_none() {
            let mut addr_list = self.build_addr_list();
            // FIXME. Agregamos nuestra direccion a la lista
            // para poder setearnos en nuestro estado interno
            // que somos el coordinador.
            addr_list.push((*self.my_address.read().unwrap()).clone());

            for n_addr in addr_list {
                self.socket
                    .send_to(COORDINATOR.to_string(), n_addr)
                    .unwrap();
            }
        }
    }

    fn build_addr_list(&self) -> Vec<String> {
        let mut addrs = vec![];
        for i in 0..MAX_NODES {
            let addr = format!("127.0.0.1:{}", 8000 + i);
            if &addr != &*self.my_address.read().unwrap() {
                addrs.push(addr);
            }
        }
        addrs
    }

    fn get_port_from_addr(&self, addr: String) -> u32 {
        addr.split(":").collect::<Vec<&str>>()[1]
            .parse::<u32>()
            .unwrap()
    }

    fn find_upper_sockets(&self) -> Vec<String> {
        let mut upper_nodes = vec![];

        for n_addr in self.build_addr_list() {
            if self.get_port_from_addr((*self.my_address.read().unwrap()).clone())
                < self.get_port_from_addr(n_addr.clone()) {
                upper_nodes.push(n_addr);
            }
        }
        upper_nodes
    }
}
