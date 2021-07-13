use super::socket::Socket;

pub struct Node {
    id: id,
    socket: Socket
}

impl Node {
    pub new(id: String, ip: String) -> Self {
        Node{
            id: id, 
            socket: Socket::new(ip, "node")
        }
    }
}