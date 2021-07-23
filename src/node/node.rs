use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::encoder::Encoder;
use crate::messages::{ACQUIRE_MSG, RELEASE_MSG, NEW_NODE_MSG};

const CTOR_ADDR: &str = "127.0.0.1:8001";

pub struct Node {
    pub writer: TcpStream,
    pub reader: BufReader<TcpStream>,
    pub leader_addr: Arc<Mutex<Option<String>>>,
    pub bully_addr: String,
}

impl Node {
    pub fn new(bully_addr: String, leader_addr: Arc<Mutex<Option<String>>>) -> Self {
        let stream = TcpStream::connect(CTOR_ADDR).unwrap();
        let writer = stream.try_clone().unwrap();
        let reader = BufReader::new(stream);
        Node { writer, reader, leader_addr, bully_addr }
    }

    pub fn run(&mut self) {
        self.fetch_leader_addr();
        for _ in 1..10 {
            self.acquire();
            thread::sleep(Duration::from_millis(1000));
            self.release();
        }
    }

    fn acquire(&mut self) {
        self.writer.write_all(ACQUIRE_MSG.as_bytes()).unwrap();
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        println!("Read {}", buffer);
    }

    fn release(&mut self) {
        self.writer.write_all(RELEASE_MSG.as_bytes()).unwrap();
    }

    fn fetch_leader_addr(&mut self) {
        // Pregunta al coordinador la IP del lider actual, si recibimos 
        // nuestra IP entonces somos nosotros.
        println!("Enviando mensaje de discovery");

        self.writer.write_all(NEW_NODE_MSG.as_bytes()).unwrap();
        self.writer.write_all(&Encoder::encode_to_bytes(self.bully_addr.as_str())).unwrap();

        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        let leader_ip = buffer.split('\n').collect::<Vec<&str>>()[0].to_string();

        (*self.leader_addr.lock().unwrap()) = Some(leader_ip);
    }
}
