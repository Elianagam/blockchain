use crate::logger::Logger;
use crate::messages::{ACQUIRE_MSG, DISCONNECT_MSG, NEW_NODE_MSG, RELEASE_MSG};
use crate::node_accepted::NodeAccepted;

use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use std_semaphore::Semaphore;

const CTOR_ADDR: &str = "127.0.0.1:8001";

pub struct Coordinator {
    socket: TcpListener,
    current_leader: Arc<Mutex<Option<String>>>,
    logger: Arc<Logger>,
}

impl Coordinator {
    pub fn new(logger: Arc<Logger>) -> Coordinator {
        let socket = TcpListener::bind(CTOR_ADDR).unwrap();
        let current_leader = Arc::new(Mutex::new(None));
        Coordinator {
            socket,
            current_leader,
            logger: logger.clone(),
        }
    }

    pub fn run(&self) {
        let mutex = Arc::new(Semaphore::new(1));

        for stream in self.socket.incoming() {
            let tcp_stream = stream.unwrap();
            let id = tcp_stream.peer_addr().unwrap().port();
            let mut node = NodeAccepted::new(tcp_stream);

            self.logger
                .info(format!("[COORDINATOR] Cliente conectado {}", id));

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
                            }
                        }
                        RELEASE_MSG => {
                            println!("[COORDINATOR] Libera lock {}", id);
                            if mine {
                                local_mutex.release();
                                mine = false;
                            }
                        }
                        NEW_NODE_MSG => {
                            println!("[COORDINATOR] Nuevo nodo conectado");
                            let new_node_bully_addr = node.read();
                            let tmp = (*current_leader).lock().unwrap().clone();
                            match tmp {
                                // Si tenemos un lider seteado devolvemos su IP
                                Some(current_leader_addr) => {
                                    println!("[COORDINATOR] Lider encontrado, devolviendolo");
                                    node.write(format!("{}\n", current_leader_addr.clone()));
                                }
                                None => {
                                    println!(
                                        "[COORDINATOR] Seteando como lider a: {:?}",
                                        new_node_bully_addr
                                    );
                                    *current_leader.lock().unwrap() =
                                        Some(new_node_bully_addr.clone());

                                    // Hacemos un echo de la IP que nos pasaron, de esta forma
                                    // el nodo puede saber que se lo asigno como lider.
                                    node.write(format!("{}\n", new_node_bully_addr));
                                }
                            }
                        }
                        DISCONNECT_MSG => {
                            println!("[COORDINATOR] Desconectado {}", id);
                            break;
                        }
                        _ => {
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
