use ::network::{Connection, Packet};
use std::net::UdpSocket;

pub struct Network {
    cxn: Connection,
}

impl Network {
    pub fn new() -> Self {
        let mut connection = Connection::connect().unwrap();
        connection
            .send(&Packet::Login { username: [5; 20] })
            .unwrap();
        Self { cxn: connection }
    }

    pub fn update(&mut self) {
        //
    }
}
