mod node;

mod blockchain;

use std::env;
use std::process;

mod encoder;
mod leader_discoverer;
mod leader_down_handler;
mod stdin_reader;
mod utils;
use utils::logger::Logger;
use std::sync::{Arc};

const MESSAGE_LOGGER_ERROR: &str = "Unable to open logger file ";


fn port_missing() -> i32 {
    println!("Number of port must be specified");
    return -1;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        process::exit(port_missing());
    }
    let logger = match Logger::new(&format!("log_{}",&args[1])) {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            println!("{} {:?}: {}", MESSAGE_LOGGER_ERROR, &args[1], e);
            process::exit(-1);
        }
    };
    println!("Logging messages will be saved to: log_{:?}.", args[1]);

    let blockchain_filename = format!("log_{}_blockchain", args[1]);

    let blockchain_logger = match Logger::new(&blockchain_filename) {
        Ok(logger) => Arc::new(logger),
        Err(e) => {
            println!("{} {:?}: {}", MESSAGE_LOGGER_ERROR, &args[1], e);
            process::exit(-1);
        }
    };

    println!("Detailed blockchain will be logged in: {:?}\n", blockchain_filename);

    let mut node = node::Node::new(&args[1], logger, blockchain_logger);
    node.run();
}
