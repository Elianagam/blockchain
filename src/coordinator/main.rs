mod coordinator;

use std::env;
use std::thread;
use std::process;

fn usage() -> i32 {
	println!("Usage: cargo r --bin coordinator");
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
        Err(e) => println!("{:?}", e)
    };

	Ok(())
}
