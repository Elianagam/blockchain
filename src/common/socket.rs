use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use super::socket::Socket;


pub struct Socket {
	pub fd: TcpStream
}

impl Socket {

	pub fn new(ip: String, type: String) -> Self {
		let socket = match type {
			"client" => Socket{ fd: connect(ip) }
			"server" => Socket{ fd: listen(ip) }
		}
		socket
	}

	pub fn write(&self, message: String) {
		let writer = self.fd.try_clone().unwrap();

		writer.write_all(message.as_bytes()).unwrap();
	}

	pub fn read(&self) -> String {
		let mut reader = BufReader::new(self.fd);

		let mut buffer = String::new();
		reader.read_line(&mut buffer);

		buffer.as_str()
	}

	 fn accept(&self) -> Socket {
		self.fd.accept().map(|(socket, _addr)| {
			Socket { fd: socket }
		})
	}

	fn listen(ip: String) -> TcpStream {
		TcpListener::bind(ip).unwrap() 
	}

	fn connect(ip: String) -> TcpStream {
		TcpStream::connect(ip).unwrap();
	}
}
