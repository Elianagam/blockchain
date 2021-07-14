#[path = "../common/socket.rs"]
mod socket;
use std::io::{BufReader,BufRead,Write};
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
            let mut writer = tcp_stream.try_clone().unwrap();
            let mut reader = BufReader::new(tcp_stream);
            let local_mutex = mutex.clone();
            let mut id = String::new();
            reader.read_line(&mut id).unwrap();
            id = id.replace("\n", "");
            println!("[COORDINATOR] Cliente conectado {}", id);
            thread::spawn(move || {
                let mut mine = false;

                loop {
                    let mut buffer = String::new();
                    reader.read_line(&mut buffer).unwrap();
                    match buffer.as_str() {
                        "acquire\n" => {
                            println!("[COORDINATOR] pide lock {}", id);
                            if !mine {
                                local_mutex.acquire();
                                mine = true;
                                writer.write_all("OK\n".as_bytes()).unwrap();
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
