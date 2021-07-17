mod node;


use std::env;

use std::process;

fn usage() -> i32 {
	println!("Usage: cargo r --bin node <id> <ip_address> ");
	return -1;
}

fn main() -> Result<(), ()> {

	let args: Vec<String> = env::args().collect();
    if args.len() != 1 {
        process::exit(usage());
    }

    let mut node = node::Node::new();
    node.run();

	Ok(())
}

/*
	let mut socket = TcpStream::connect(address).unwrap();
	println!("Conectando. Ingrese texto");

	for line in std::io::stdin().lock().lines() {
		socket.write(line.unwrap().as_bytes()).unwrap();
		socket.write("\n".as_bytes()).unwrap();
	}
*/