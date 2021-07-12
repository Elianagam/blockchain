use std::io::{BufRead, BufReader};
use std::net::TcpListener;



pub struct Coordinator {
    listener: TcpListener,
}

impl Coordinator {
    pub fn new(address: String) -> Coordinator {
        let mut ret = Coordinator {
            listener: TcpListener::bind(address).unwrap();
        };
        ret
    }

    pub fn accept(&self) -> Socket {
        self.listener.accept().map(|(socket, _addr)| {
            Socket { fd: socket }
        })
    }

    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));

        loop {
            let new_socket = self.accept();

            let buffer = new_socket.read();
            match buffer {
                "BUSY\n" => {
                    println!("[COORDINATOR] pide lock");
                }
                "OK\n" => {
                    println!("[COORDINATOR] libera lock");
                }
                "" => {
                    println!("[COORDINATOR] desconectado");
                    break;
                }
            }
        }   
    }

    fn loop(&self, reader) {
        let mut buffer = String::new();
                reader.read_line(&mut buffer);
    }
}