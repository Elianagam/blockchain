mod coordinator;

use std::env;
use std::thread;
use std::process;

fn usage() -> i32 {
	println!("Usage: cargo r --bin coordinator <ip_address> ");
	return -1;
}


fn main() -> Result<(), ()> {
	let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        process::exit(usage());
    }

    let ip = args[1].clone();
    let coordinator = thread::spawn(move || {
    	let coordinator = coordinator::Coordinator::new(ip);
    	coordinator.run();
	});


	match coordinator.join() {
        Ok(()) => println!("Join Coordinator"),
        Err(e) => println!("{:?}", e)
    };

	Ok(())
}

/*
let address = "0.0.0.0:10000".to_string();

	let listener = TcpListener::bind(address).unwrap();

	// get client
	let (stream, _addr) = listener.accept().unwrap();
	let reader = BufReader::new(stream);
	let mut lines = reader.lines();

	while let Some(line) = lines.next() {
		println!("Recibido: {}", line.unwrap());
	}

	*/