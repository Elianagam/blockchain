use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, TcpListener, SocketAddr, UdpSocket};
use std::time::Duration;
use std::sync::Arc;
use std::thread;
use std::env;
use std::str;
use std::process;

mod node;

const LEADER_ADDR: &str = "127.0.0.1:8000";

fn run_bully_as_non_leader(mut blockchain: Vec<String>) {
	// Let the OS to pick one addr + port for us
	let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

	socket.send_to("register\n".as_bytes(), LEADER_ADDR).unwrap();

	for _ in 0..3 {
		let mut buf = [0; 128];
		let (size, from) = socket.recv_from(&mut buf).unwrap();
		let msg = str::from_utf8(&buf).unwrap();
		println!("Recibido {:?}", &msg);
		blockchain.push(msg.to_string());
	}
}

fn run_bully_as_leader(mut blockchain: Vec<String>) {
	println!("Soy el líder!");

	let mut other_nodes: Vec<SocketAddr> = vec!();

	let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

	let mut message_count = 3;

	loop {
		let mut buf = [0; 128];
		let (size, from) = socket.recv_from(&mut buf).unwrap();

        let msg = str::from_utf8(&buf)
            .unwrap()
            .split("\n")
            .collect::<Vec<&str>>()[0];

		match msg {
			"register" => {
				println!("Registrando nodo: {}", from);
				if !&other_nodes.contains(&from) {
					other_nodes.push(from);
				}
			},
			msg => {
				println!("Propagando cambios {:?} al resto de los nodos", msg);
				for node in &other_nodes {
					socket.send_to(msg.as_bytes(), node).unwrap();
				}
				blockchain.push(msg.to_string());
				message_count -= 1;
			}
		}
	}
}

fn run_bully_thread(iamleader: bool) -> () {
	let blockchain: Vec<String> = vec!(); 

	match iamleader {
		true => run_bully_as_leader(blockchain),
		false => run_bully_as_non_leader(blockchain)
	}
}

fn usage() -> i32 {
	println!("Usage: cargo r --bin node <id> <ip_address> ");
	return -1;
}


fn main() -> Result<(), ()> {
	let args: Vec<String> = env::args().collect();


	let iamleader: bool = args.len() > 1 && args[1] == "--leader";
	let t = thread::spawn(move || run_bully_thread(iamleader));

	t.join();

	return Ok(());

	let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        process::exit(usage());
    }

	let id = &args[1];
	let address = &args[2];

    let mut node = node::Node::new(id.to_string(), address.to_string());
    node.run();

	Ok(())
}
