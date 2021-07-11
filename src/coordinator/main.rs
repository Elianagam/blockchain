use std::io::{BufRead, BufReader};
use std::net::TcpListener;

fn main() -> Result<(), ()> {
	let address = "0.0.0.0:10000".to_string();

	let listener = TcpListener::bind(address).unwrap();

	// get client
	let (stream, _addr) = listener.accept().unwrap();
	let reader = BufReader::new(stream);
	let mut lines = reader.lines();

	while let Some(line) = lines.next() {
		println!("Recibido: {}", line.unwrap());
	}

	Ok(())
}