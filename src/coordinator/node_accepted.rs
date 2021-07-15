use std::net::TcpStream;
use std::io::{BufReader,BufRead,Write};


pub struct NodeAccepted {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    pub id: String
}


impl NodeAccepted {
    pub fn new(stream: TcpStream) -> NodeAccepted {
        let writer = stream.try_clone().unwrap();
        let mut reader = BufReader::new(stream);
        
        let mut id = String::new();
        reader.read_line(&mut id).unwrap();
        id = id.to_string().replace("\n", "");
        
        NodeAccepted{writer: writer, reader: reader, id: id}
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