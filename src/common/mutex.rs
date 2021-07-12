use super::socket::Socket;

pub struct Mutex;

impl Mutex {
    pub fn acquire(&mut self, socket: Socket) {
        socket.write("BUSY".to_string());
    }

    pub fn release(&mut self, socket: Socket) {
        socket.write("OK".to_string());
    }
}