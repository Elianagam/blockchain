#[path = "node_accepted.rs"]
mod node_accepted;
use node_accepted::NodeAccepted;

use std::net::TcpListener;
use std::sync::Arc;
use std_semaphore::Semaphore;
use std::thread::{self};


pub struct Coordinator {
    socket: TcpListener,
}


impl Coordinator {
    pub fn new(ip: String) -> Coordinator {
        Coordinator {
            socket: TcpListener::bind(ip).unwrap(),
        }
    }

    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));

        for stream in self.socket.incoming() {
            let tcp_stream = stream.unwrap();
            let mut node = NodeAccepted::new(tcp_stream);
            let id = node.id.to_string();
            println!("[COORDINATOR] Cliente conectado {}", id);

            let local_mutex = mutex.clone();

            thread::spawn(move || {
                let mut mine = false;

                loop {
                    let buffer = node.read();
                    match buffer.as_str() {
                        "acquire\n" => {
                            println!("[COORDINATOR] pide lock {}", id);
                            if !mine {
                                local_mutex.acquire();
                                mine = true;
                                node.write("OK\n".to_string());
                                println!("[COORDINATOR] le dí lock a {}", id);
                            } else {
                                println!("[COORDINATOR] ERROR: ya lo tiene");
                            }
                        }
                        "release\n" => {
                            println!("[COORDINATOR] libera lock {}", id);
                            if mine {
                                local_mutex.release();
                                mine = false;
                            } else {
                                println!("[COORDINATOR] ERROR: no lo tiene!")
                            }
                        }
                        "" => {
                          println!("[COORDINATOR] desconectado {}", id);
                          break;
                        }
                        _ => {
                            println!("[COORDINATOR] ERROR: mensaje desconocido de {}", id);
                            break;
                        }
                    }
                }
                if mine {
                    println!("[COORDINATOR] ERROR: tenia el lock");
                    local_mutex.release();
                }
            });
        }
    }
}
