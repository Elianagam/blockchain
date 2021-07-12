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

	pub fn read(&self) {
		let mut buffer = String::new();
        self.fd.read_line(&mut buffer);
	}
}
