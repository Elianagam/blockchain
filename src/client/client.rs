use std::io::Write;
use std::io::BufRead;
use std::net::TcpStream;

fn main() -> Result<(), ()> {
	let address = "0.0.0.0:10000".to_string();

	let mut socket = TcpStream::connect(address).unwrap();
	println!("Conectando. Ingrese texto");

	for line in std::io::stdin().lock().lines() {
		socket.write(line.unwrap().as_bytes()).unwrap();
		socket.write("\n".as_bytes()).unwrap();
	}

	Ok(())
}