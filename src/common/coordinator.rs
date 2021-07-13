use std::io::{BufRead, BufReader};
use std::net::TcpListener;



pub struct Coordinator {
    socket: Socket,
}

impl Coordinator {
    pub fn new(ip: String) -> Coordinator {
        let mut ret = Coordinator {
            socket: Socket::new(ip, "coordinator")
        };
        ret
    }

    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));

        loop {
            let new_socket = self.socket.accept();

            let buffer = new_socket.read();
            match buffer {
                "adquire\n" => {
                    println!("[COORDINATOR] pide lock");
                }
                "release\n" => {
                    println!("[COORDINATOR] libera lock");
                }
                "" => {
                    println!("[COORDINATOR] desconectado");
                    break;
                }
            }
        }   
    }
}
