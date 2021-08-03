use crate::utils::messages::*;
use std::io::{self, BufRead};
use std::option::Option;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use crate::blockchain::blockchain::Blockchain;
use crate::utils::logger::Logger;
use crate::utils::messages::*;
use crate::utils::socket::Socket;

const ACK_TIMEOUT_SECS: u64 = 2;
const WAITING_FOR_LOCK_ACQUIRED_TIMEOUT: u64 = 15;

/// Responsible for read msg from stdin with diferent options
/// 
pub struct StdinReader {
    leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    socket: Socket,
    leader_addr: Arc<RwLock<Option<String>>>,
    node_alive: Arc<RwLock<bool>>,
    msg_ack_cv: Arc<(Mutex<bool>, Condvar)>,
    leader_down_cv: Arc<(Mutex<bool>, Condvar)>,
    lock_acquired: Arc<(Mutex<bool>, Condvar)>,
    blockchain: Arc<RwLock<Blockchain>>,
    blockchain_logger: Arc<Logger>
}

impl StdinReader {
    pub fn new(
        leader_condvar: Arc<(Mutex<bool>, Condvar)>,
        socket: Socket,
        leader_addr: Arc<RwLock<Option<String>>>,
        node_alive: Arc<RwLock<bool>>,
        msg_ack_cv: Arc<(Mutex<bool>, Condvar)>,
        leader_down_cv: Arc<(Mutex<bool>, Condvar)>,
        lock_acquired: Arc<(Mutex<bool>, Condvar)>,
        blockchain: Arc<RwLock<Blockchain>>,
        blockchain_logger: Arc<Logger>
    ) -> Self {
        StdinReader {
            leader_condvar,
            socket,
            leader_addr,
            node_alive,
            msg_ack_cv,
            leader_down_cv,
            lock_acquired,
            blockchain,
            blockchain_logger
        }
    }

    /// Print menu string with options
    fn menu(&self) {
        println!("Select an option:\n\t1. Add block\n\t2. Print Blockchain\n\t3. Exit");
    }

    /// Await until leader is set and read from stdin
    /// If the msg is a new block adquire mutex and 
    /// sent that msg to leader addr if the mutex is taken
    /// Sent the block and realese mutex
    pub fn run(&mut self) {
        loop {
            self.wait_for_leader();
            let value = self.read_option();
            if &value == "" {
                continue;
            }
            if &value == CLOSE {
                let mut guard = self.node_alive.write().unwrap();
                *guard = false;
                let me = self.socket.local_addr().to_string();
                self.socket.send_to(NOOP_MSG.to_string(), me).unwrap();
                break;
            }

            let addr = self.leader_addr.read().unwrap().clone();
            if addr.is_none() {
                continue;
            }

            // Tomamos el lock del leader
            self.socket
                .send_to(ACQUIRE_MSG.to_string(), addr.clone().unwrap())
                .unwrap();

            // Asumimos que no hay congestion mas de WAITING_FOR_LOCK_ACQUIRED_TIMEOUT
            // Esperamos en la condvar hasta recibir un mensaje de LOCK_AQUIRED
            {
                let (lock, cvar) = &*self.lock_acquired;
                let guard  = lock.lock().unwrap();
                let timeout = Duration::from_secs(WAITING_FOR_LOCK_ACQUIRED_TIMEOUT);
                let result = cvar
                    .wait_timeout_while(guard, timeout, |&mut lock_acquired| !lock_acquired)
                    .unwrap();

                if result.1.timed_out() {
                    // El lider no nos dió el lock en WAITING_FOR_LOCK_ACQUIRED_TIMEOUT
                    // puede estar caído o simplemente hay mucha congestión.
                    println!("Operación fallida reintentar");
                    self.set_leader_down();
                    continue;
                }
            }
            
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
            let mut data_to_send = String::new();
            data_to_send.push_str(
                &(format!(
                    "{},{},{}",
                    &value,
                    &now,
                    &self.socket.local_addr().to_string()
                ))
            );
            // Nos dieron el lock
            self.socket.send_to(data_to_send, addr.clone().unwrap()).unwrap();

            self.wait_for_ack();

            self.socket
                .send_to(RELEASE_MSG.to_string(), addr.clone().unwrap())
                .unwrap();
        }
    }

    /// Read a new line from stdin
    fn read(&self) -> String {
        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        let line = iterator.next().unwrap().unwrap();
        line
    }

    /// If option to add new block was choseen 
    /// then read again from stdin and return value if is valis
    fn option_add_block(&mut self) -> String {
        println!("Write a block (id,qualification): ");
        let line = self.read();
        let student_data: Vec<&str> = line.split(",").collect();
        if student_data.len() != 2 {
            println!("Unsupported data format, usage: id, qualification")
        }
        return line.to_string();
    }

    /// Read Menu option input from stdin
    fn read_option(&mut self) -> String {
        self.menu();
        let option = self.read();

        match option.as_str() {
            "1" => return self.option_add_block(),
            "2" => self.option_show_blockchain(),
            "3" => return CLOSE.to_string(),
            _ => {
                println!("Invalid option, choose again...")
            }
        }; 

        return String::new();
    }

    fn option_show_blockchain(&self) {
        let blockchain = self.blockchain.read().unwrap().clone();
        println!("{}", blockchain);
        
        for block in blockchain.blocks {
            self.blockchain_logger.info(format!("{:#?}\n", block));
        }
    }

    /// Await for leadr addr is set 
    /// Is notificated with a leader condvar
    fn wait_for_leader(&self) {
        let (lock, cv) = &*self.leader_condvar;

        let mut leader_found = lock.lock().unwrap();

        while !*leader_found {
            leader_found = cv.wait(leader_found).unwrap();
        }
    }

    fn wait_for_ack(&self) {
        let (lock, cv) = &*self.msg_ack_cv;
        let mut guard = lock.lock().unwrap();

        // TODO: Si esperar el ack nos da timeout es porque el lider
        // esta caido. Esperar a que se setee el nuevo lider y reintentar
        //TODO. Agregar guard para los spurious wake up
        let result = cv
            .wait_timeout(guard, Duration::from_secs(ACK_TIMEOUT_SECS))
            .unwrap();

        guard = result.0;

        if !*guard { self.set_leader_down() }

        *guard = false;
    }

    /// If found that the leader is down change
    ///  value of condvar and notify all nodes
    fn set_leader_down(&self) {
        let (lock_leader_down, cv_leader_down) = &*self.leader_down_cv;
        *lock_leader_down.lock().unwrap() = true;
        cv_leader_down.notify_all();
    }
}
