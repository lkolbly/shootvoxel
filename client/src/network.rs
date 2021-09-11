use ::network::Packet;
use std::net::UdpSocket;

pub struct Network {
    socket: UdpSocket,
}

impl Network {
    pub fn new() -> Self {
        let mut socket = UdpSocket::bind("127.0.0.1:3421").unwrap();
        socket.connect("127.0.0.1:3420").unwrap();
        socket.set_nonblocking(true).unwrap();
        let packet: Vec<u8> = bincode::serialize(&Packet::Login { username: [5; 20] }).unwrap();
        socket.send(&packet).unwrap();
        Self { socket }
    }

    pub fn update(&mut self) {
        //
    }
}
