#[path = "node_accepted.rs"]
mod node_accepted;
use node_accepted::NodeAccepted;

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std_semaphore::Semaphore;

const CTOR_ADDR: &str = "127.0.0.1:8001";
const ACQUIRE_MSG: &str = "acquire\n";
const RELEASE_MSG: &str = "release\n";
const NEW_NODE_MSG: &str = "discover\n";
const DISCONNECT_MSG: &str = "";

pub struct Coordinator {
    socket: TcpListener,
    current_leader: Arc<Mutex<Option<String>>>,
}

impl Coordinator {
    pub fn new() -> Coordinator {
        Coordinator {
            socket: TcpListener::bind(CTOR_ADDR).unwrap(),
            current_leader: Arc::new(Mutex::new(None)),
        }
    }

    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));

        for stream in self.socket.incoming() {
            let tcp_stream = stream.unwrap();
            let id = tcp_stream.peer_addr().unwrap().port();
            let mut node = NodeAccepted::new(tcp_stream);
            println!("[COORDINATOR] Cliente conectado {}", id);

            let local_mutex = mutex.clone();

            let current_leader = self.current_leader.clone();

            std::thread::spawn(move || {
                let mut mine = false;

                loop {
                    let buffer = node.read();
                    match buffer.as_str() {
                        ACQUIRE_MSG => {
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
                        RELEASE_MSG => {
                            println!("[COORDINATOR] libera lock {}", id);
                            if mine {
                                local_mutex.release();
                                mine = false;
                            } else {
                                println!("[COORDINATOR] ERROR: no lo tiene!")
                            }
                        }
                        DISCONNECT_MSG => {
                            println!("[COORDINATOR] desconectado {}", id);
                            break;
                        }
                        NEW_NODE_MSG => match (*current_leader).lock().unwrap().clone() {
                            Some(leader_id) => node.write(leader_id),
                            None => {}
                        },
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
