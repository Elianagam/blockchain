mod coordinator;

use std::env;
use std::process;
use std::thread;

fn usage() -> i32 {
    println!("Usage: cargo r --bin coordinator <ip_address> ");
    return -1;
}

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 1 {
        process::exit(usage());
    }

    let coordinator = thread::spawn(move || {
        let coordinator = coordinator::Coordinator::new();
        coordinator.run();
    });

    match coordinator.join() {
        Ok(()) => println!("Join Coordinator"),
        Err(e) => println!("{:?}", e),
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
