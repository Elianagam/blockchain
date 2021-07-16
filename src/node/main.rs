use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::env;
use std::process;
use std::str;

mod node;

const LEADER_ADDR: &str = "127.0.0.1:8000";
const DUMMY_MSG: &str = "testing";
const REGISTER_MSG: &str = "register";
const MSG_EOF: char = '\n';

fn encode_to_bytes(msg: &str) -> Vec<u8> {
    let mut message = String::from(msg.clone());
    message.push(MSG_EOF);
    message.into_bytes()
}

fn decode_from_bytes(payload: Vec<u8>) -> String {
    let data = str::from_utf8(&payload)
        .unwrap()
        .split(MSG_EOF)
        .collect::<Vec<&str>>()[0];
    data.to_string()
}

fn run_bully_as_non_leader(mut blockchain: Vec<String>) {
	// Let the OS to pick one addr + port for us
	let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

	socket.send_to(&encode_to_bytes(REGISTER_MSG), LEADER_ADDR).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("Enviando mensaje {} al lider", DUMMY_MSG);

	socket.send_to(&encode_to_bytes(DUMMY_MSG), LEADER_ADDR).unwrap();

	for _ in 0..3 {
		let mut buf = [0; 128];
		let (_, _) = socket.recv_from(&mut buf).unwrap();

        let msg = decode_from_bytes(buf.to_vec());

		println!("Recibido {}", &msg);

		blockchain.push(msg);
	}

    println!("Blockchain final: {:?}", blockchain);
}

fn run_bully_as_leader(mut blockchain: Vec<String>) {
	println!("Soy el líder!");

	let mut other_nodes: Vec<SocketAddr> = vec!();

	let socket = UdpSocket::bind(LEADER_ADDR).unwrap();

    let mut propagated_msgs = 0;

	loop {
		let mut buf = [0; 128];
		let (_, from) = socket.recv_from(&mut buf).unwrap();

        let msg = decode_from_bytes(buf.to_vec());

        if propagated_msgs == 3 { break }

		match msg.as_str() {
			REGISTER_MSG => {
				println!("Registrando nodo: {}", from);
				if !&other_nodes.contains(&from) {
					other_nodes.push(from);
				}
			},
			msg => {
				println!("Propagando cambios {:?} al resto de los nodos", msg);
				for node in &other_nodes {
					socket.send_to(&encode_to_bytes(msg), node).unwrap();
				}
				blockchain.push(msg.to_string());
                propagated_msgs += 1;
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
	println!("Usage: cargo r --bin node <id> <ip_address> [--leader]");
	return -1;
}


fn main() -> Result<(), ()> {
	let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        process::exit(usage());
    }

	let id = &args[1];
	let address = &args[2];

	let iamleader: bool = args.len() > 3 && args[3] == "--leader";
	let t = thread::spawn(move || run_bully_thread(iamleader));

    let mut node = node::Node::new(id.to_string(), address.to_string());

    node.run();

	t.join().unwrap();

	Ok(())
}
