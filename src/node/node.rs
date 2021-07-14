use core::time::Duration;
use std::io::{BufReader,BufRead,Write};
use std::net::{TcpStream};
use std::thread;

pub struct Node {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    id: String
}

impl Node {
    pub fn new(id: String, ip_address: String) -> Self {
        let stream = TcpStream::connect(ip_address).unwrap();
        let mut ret = Node {
            writer: stream.try_clone().unwrap(),
            reader: BufReader::new(stream),
            id: id.to_string()
        };

        ret.writer.write_all((id.to_string() + "\n").as_bytes() ).unwrap();

        ret
    }

    pub fn run(&mut self) {
        loop {
            println!("[{}] pidiendo lock", self.id);

            self.acquire();
            println!("[{}] tengo el lock", self.id);
            thread::sleep(Duration::from_millis(100));
            println!("[{}] libero el lock", self.id);
            self.release();
        }
    }

    fn acquire(&mut self) {
        self.writer.write_all("acquire\n".as_bytes()).unwrap();

        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();
    }

    fn release(&mut self) {
        self.writer.write_all("release\n".as_bytes()).unwrap();
    }
}