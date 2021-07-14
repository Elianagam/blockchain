use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use super::node::Node;
use super::mutex::Mutex;
use super::socket::Socket;



pub struct Coordinator {
    socket: Socket,
    mutex: Mutex
}

impl Coordinator {
    pub fn new(ip: String) -> Coordinator {
        let mut ret = Coordinator {
            socket: Socket::new(ip, "coordinator")
            mutex: Arc<Mutex::new()>
        };
        ret
    }


    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));
        let connected = Vec::new();

        loop {
            // socket accept new client
            let new_socket = self.socket.accept();
            println!("Coordinator accept new Node-Client");
            connected.push(new_socket);

            // create Node with socket acceptor
            let id = new_socket.read();
            println!("[COORDINATOR] Cliente conectado {}", id);
            let local_mutex = mutex.clone();


            thread::spawn(move || {
                let mut mine = false;

                let buffer = new_socket.read();
                match buffer {
                    "adquire\n" => {
                        println!("[COORDINATOR] pide lock");
                        if !mine {
                            local_mutex.acquire();
                            mine = true;
                            self.socket.write(format!("OK\n"));
                            println!("[COORDINATOR] le dí lock a {}", id);
                        }
                    }
                    "release\n" => {
                        println!("[COORDINATOR] libera lock");
                        if mine {
                            local_mutex.release();
                            mine = false;
                        }
                    }
                    "" => {
                        println!("[COORDINATOR] desconectado");
                        break;
                    }
                }
            }
        }   
    }
}
