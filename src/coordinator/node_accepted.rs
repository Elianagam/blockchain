use std::net::TcpStream;
use std::io::{BufReader,BufRead,Write};


pub struct NodeAccepted {
    writer: TcpStream,
    reader: BufReader<TcpStream>
}


impl NodeAccepted {
    pub fn new(stream: TcpStream) -> NodeAccepted {
        let writer = stream.try_clone().unwrap();
        let reader = BufReader::new(stream);
        NodeAccepted{writer: writer, reader: reader}
    }

    pub fn write(&mut self, message: String) {
        self.writer.write_all(message.as_bytes()).unwrap();
    }

    pub fn read(&mut self) -> String{
        let mut buffer = String::new();
        self.reader.read_line(&mut buffer).unwrap();

        buffer.to_string()
    }
}