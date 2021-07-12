use std::io::{BufRead, BufReader};
use std::net::TcpListener;



pub struct Coordinator {
    listener: TcpListener,
}

impl Coordinator {
    pub fn new(address: String) -> Coordinator {
		let mut ret = Coordinator {
            listener: TcpListener::bind(address).unwrap();
        };
        ret
    }

    pub fn accept(&self) -> Socket {
    	self.listener.accept().map(|(socket, _addr)| {
            Socket { fd: socket }
        })
    }


    /*
            let tcp_stream = stream.unwrap();
        let mut writer = tcp_stream.try_clone().unwrap();
        let mut reader = BufReader::new(tcp_stream);
        let local_mutex = mutex.clone();
        */ 
}