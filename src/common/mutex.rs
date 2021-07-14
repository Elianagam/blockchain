use std_semaphore::Semaphore;
use super::socket::Socket;

#[derive(Clone)]
pub struct Mutex;

impl Mutex {
    pub fn acquire(&mut self, socket: Socket) {
        socket.write("adquire\n".to_string());
    }

    pub fn release(&mut self, socket: Socket) {
        socket.write("release\n".to_string());
    }
}
