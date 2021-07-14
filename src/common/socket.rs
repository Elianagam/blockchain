use std::net::TcpStream;
use std::io::{BufReader, BufRead, Write};
use std::net::TcpListener;

#[derive(Clone)]
pub struct Socket {
	fd: TcpStream
}

impl Socket {
	pub fn new(fd: TcpStream) -> Self {
		Socket{fd: fd}
	}

	pub fn write(&self, message: String) {
		let writer = self.fd.try_clone().unwrap();

		writer.write_all(message.as_bytes()).unwrap();
	}

	pub fn read(&self) -> &str {
		let tcp_stream = self.fd;
		let mut reader = BufReader::new(tcp_stream);

		let mut buffer = String::new();
		reader.read_line(&mut buffer);

		buffer.as_str()
	}
}


pub struct SocketServer {
	pub fd: TcpListener
}

impl SocketServer {
	pub fn new(ip: String) -> Self {
		let socket = SocketServer{ fd: TcpListener::bind(ip).unwrap() };
		socket
	}

	pub fn accept(&self) -> Socket {
		let socket = self.fd.accept().map(|(new_socket, _addr)| {
			Socket { fd: new_socket }
		});
		socket.unwrap()
	}
}