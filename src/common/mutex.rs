use std_semaphore::Semaphore;
use super::socket::Socket;

pub struct Mutex {
    pub socket: Socket
}

impl Mutex {
    pub fn acquire(&mut self) {
        self.socket.write("adquire\n".to_string());
    }

    pub fn release(&mut self) {
        self.socket.write("release\n".to_string());
    }
}
