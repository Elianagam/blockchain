use std::io::{self, BufRead};
use std::option::Option;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;

use crate::utils::messages::CLOSE;
use crate::utils::socket_with_timeout::SocketWithTimeout;

const ACK_TIMEOUT_SECS: u64 = 2;

pub struct StdinReader {
    leader_condvar: Arc<(Mutex<bool>, Condvar)>,
    socket: SocketWithTimeout,
    leader_addr: Arc<RwLock<Option<String>>>,
    node_alive: Arc<RwLock<bool>>,
    msg_ack_cv: Arc<(Mutex<bool>, Condvar)>,
    leader_down_cv: Arc<(Mutex<bool>, Condvar)>,
}

impl StdinReader {
    pub fn new(
        leader_condvar: Arc<(Mutex<bool>, Condvar)>,
        socket: SocketWithTimeout,
        leader_addr: Arc<RwLock<Option<String>>>,
        node_alive: Arc<RwLock<bool>>,
        msg_ack_cv: Arc<(Mutex<bool>, Condvar)>,
        leader_down_cv: Arc<(Mutex<bool>, Condvar)>,
    ) -> Self {
        StdinReader {
            leader_condvar,
            socket,
            leader_addr,
            node_alive,
            msg_ack_cv,
            leader_down_cv,
        }
    }

    fn read_stdin(&mut self) -> String {
        println!("Write a student note:");
        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        let line = iterator.next().unwrap().unwrap();

        let student_data: Vec<&str> = line.split(",").collect();
        if (student_data.len() == 1 && (student_data[0] == CLOSE)) || student_data.len() == 2 {
            return line.to_string();
        } else {
            println!("Unsupported data format, usage: id, qualification")
        }
        return String::new();
    }

    pub fn run(&mut self) {
        loop {
            let (lock, cv) = &*self.leader_condvar;
            {
                let mut leader_found = lock.lock().unwrap();
                while !*leader_found {
                    leader_found = cv.wait(leader_found).unwrap();
                }
            }

            let value = self.read_stdin();
            if &value == CLOSE {
                let mut guard = self.node_alive.write().unwrap();
                *guard = false;
                break;
            }
            let addr = self.leader_addr.read().unwrap().clone();
            if addr.is_none() {
                continue;
            }

            self.socket.send_to(value, addr.unwrap()).unwrap();

            let (lock, cv) = &*self.msg_ack_cv;
            let mut guard = lock.lock().unwrap();

            //TODO. add guard for spurious wake up
            let result = cv
                .wait_timeout(guard, Duration::from_secs(ACK_TIMEOUT_SECS))
                .unwrap();

            guard = result.0;

            if !*guard {
                let (lock_leader_down, cv_leader_down) = &*self.leader_down_cv;
                let mut guard_leader_down = lock_leader_down.lock().unwrap();

                *guard_leader_down = true;
                cv_leader_down.notify_all();
            }
            *guard = false;
        }
    }
}
