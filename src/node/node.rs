use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

const CTOR_ADDR: &str = "127.0.0.1:8001";

pub struct Node {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Node {
    pub fn new() -> Self {
        let stream = TcpStream::connect(CTOR_ADDR).unwrap();
        let ret = Node {
            writer: stream.try_clone().unwrap(),
            reader: BufReader::new(stream),
        };
        ret
    }

    pub fn run(&mut self) {
        for _ in 1..10 {
            self.acquire();
            thread::sleep(Duration::from_millis(1000));
            self.release();
        }
    }

    fn acquire(&mut self) {
        self.writer.write_all("acquire\n".as_bytes()).unwrap();
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
        println!("Read {}", buffer);
    }

    fn release(&mut self) {
        self.writer.write_all("release\n".as_bytes()).unwrap();
    }
}
