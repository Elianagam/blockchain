use std::net::TcpStream;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;

pub struct Socket;


impl Socket {
	pub fn write(fd: TcpStream, message: String) {
		let writer = self.fd.try_clone().unwrap();

		writer.write_all(message.as_bytes()).unwrap();
	}

	pub fn read(fd: TcpStream) -> &str {
		let mut reader = BufReader::new(self.fd);

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
		let socket = Socket{ fd: listen(ip) };
		socket
	}

	pub fn accept(&self) -> Socket {
		self.fd.accept().map(|(socket, _addr)| {
			Socket { fd: socket }
		})
	}

	fn listen(ip: String) -> TcpListener {
		TcpListener::bind(ip).unwrap() 
	}
}


pub struct SocketClient {
	pub fd: TcpStream
}

impl SocketClient {

	pub fn new(ip: String) -> Self {
		let socket = Socket{ fd: listen(ip) };
		socket
	}

	fn connect(ip: String) -> TcpStream {
		TcpStream::connect(ip).unwrap()
	}
}
