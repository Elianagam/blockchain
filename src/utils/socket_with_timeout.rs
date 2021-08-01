use std::net::{SocketAddr, UdpSocket};

use crate::encoder::{encode_to_bytes, decode_from_bytes};

const RECV_BUF_SIZE: usize = 128;

pub struct SocketWithTimeout {
    socket: UdpSocket, 
}

impl SocketWithTimeout {
    pub fn new(socket: UdpSocket) -> Self {
        SocketWithTimeout { socket }
    }

    pub fn try_clone(&mut self) -> SocketWithTimeout {
        let clone = self.socket.try_clone().unwrap();
        SocketWithTimeout::new(clone)
    }
 
    pub fn send_to(&mut self, msg: String, addr: String) -> Result<usize, std::io::Error> {
        self.socket.send_to(&encode_to_bytes(msg.as_str()), addr)
    }

    pub fn local_addr(&mut self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }

    pub fn recv_from(&mut self) -> (usize, SocketAddr, String) {
        let mut buf = [0; RECV_BUF_SIZE];
        let (size, from) = self.socket.recv_from(&mut buf).unwrap();
        (size, from, decode_from_bytes(buf.to_vec()))
    }
}
