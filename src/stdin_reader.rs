use std::net::UdpSocket;
use std::option::Option;
use std::io::{self, BufRead};
use std::sync::{RwLock, Arc};

use crate::utils::messages::CLOSE;
use crate::encoder::encode_to_bytes;


pub struct StdinReader{
    socket: UdpSocket,
    leader_addr: std::option::Option<std::string::String>,
    node_alive: Arc<RwLock<bool>>
}

impl StdinReader {
    pub fn new(socket: UdpSocket, leader_addr: Option<std::string::String>, node_alive: Arc<RwLock<bool>>) -> Self {
        StdinReader{socket, leader_addr, node_alive}
    }

    fn read_stdin(&self) -> String {
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
    
    pub fn run(&self) {
        let mut clone_addr = self.leader_addr.clone();
        loop {
            let value = self.read_stdin();
            let addr = format!("{}", clone_addr.get_or_insert("Error".to_string()));
            if &value == CLOSE { 
                let mut guard = self.node_alive.write().unwrap();
                *guard = false;
                break; 
            }
            self.socket
                    .send_to(&encode_to_bytes(&value), addr)
                    .unwrap();
        }
    }
}
