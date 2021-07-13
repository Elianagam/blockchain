use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, TcpListener, SocketAddr, UdpSocket};
use std::time::Duration;
use std::sync::Arc;
use std::thread;
use std::env;
use std::str;

const LEADER_ADDR: &str = "127.0.0.1:8000";

fn run_bully_as_non_leader() {
	// Let the OS to pick one addr + port for us
	let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

	socket.send_to("REGISTRAR".as_bytes(), LEADER_ADDR).unwrap();

	for _ in 0..3 {
		let mut buf = [0; 128];
		let (size, from) = socket.recv_from(&mut buf).unwrap();
		println!("Recibido {:?}", str::from_utf8(&buf).unwrap());
	}
}

fn run_bully_as_leader() {
	println!("Soy el líder!");

	let mut other_nodes: Vec<SocketAddr> = vec!();

	let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

	let mut message_count = 3;

	loop {
		let mut buf = [0; 10];
		let (size, from) = socket.recv_from(&mut buf).unwrap();
		println!("recibido {:?}", str::from_utf8(&buf));

		if str::from_utf8(&buf).unwrap() == "REGISTRAR\0" {
			println!("Registrando nodo {}", from);
			if !&other_nodes.contains(&from) {
				other_nodes.push(from);
			}
			continue;
		} 

		println!("Propagando cambios al resto de los nodos");

		message_count -= 1;

		for node in &other_nodes {
			socket.send_to("NUEVO CAMBIO".as_bytes(), node).unwrap();
		}
	}
}

fn run_bully_thread(iamleader: bool) -> () {
	if iamleader {
		run_bully_as_leader()
	}
	run_bully_as_non_leader()
}


fn main() -> Result<(), ()> {
	let address = "0.0.0.0:10000".to_string();

	let args: Vec<String> = env::args().collect();

	let iamleader: bool = args.len() > 1 && args[1] == "--leader";

	let t = thread::spawn(move || run_bully_thread(iamleader));

	t.join();

	return Ok(());

	let mut socket = TcpStream::connect(address).unwrap();
	println!("Conectando. Ingrese texto");

	for line in std::io::stdin().lock().lines() {
		socket.write(line.unwrap().as_bytes()).unwrap();
		socket.write("\n".as_bytes()).unwrap();
	}

	Ok(())
}
