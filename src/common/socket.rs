use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use super::socket::Socket;


pub struct Socket {
	pub fd: TcpStream
}

impl Socket {
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

	pub fn accept(&self) -> Socket {
		self.listener.accept().map(|(socket, _addr)| {
			Socket { fd: socket }
		})
	}
}
