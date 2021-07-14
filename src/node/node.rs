use super::socket::SocketClient;

pub struct Node {
    id: id,
    socket: Socket
    mutex_message: Mutex
}

impl Node {
    pub fn new(id: String, ip: String) -> Self {
        let socket = Socket::new(TcpStream::connect(ip).unwrap());
        let node = Node{
            id: id, 
            socket: socket, 
            mutex: Mutex{socket: socket}
        }
        socket.write(id);
        println!("[{}] conectado", id);
        node
    }

    pub fn run(&self) {
        loop {
            println!("[{}] durmiendo", self.id);
            thread::sleep(Duration::from_millis(thread_rng().gen_range(1000, 3000)));
            println!("[{}] pidiendo lock", self.id);

            self.mutex_message.acquire();
            println!("[{}] tengo el lock", self.id);
            thread::sleep(Duration::from_millis(thread_rng().gen_range(1000, 3000)));
            println!("[{}] libero el lock", self.id);
            self.mutex_message.release();
        }
    }
}