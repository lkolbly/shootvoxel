use network::Packet;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;

const MTU: usize = 508;

fn main() {
    let mut socket = UdpSocket::bind("127.0.0.1:3420").unwrap();

    loop {
        let mut buf = [0; MTU];
        let (amt, src) = socket.recv_from(&mut buf).unwrap();
        let buf = &mut buf[..amt];

        let decoded: Packet = bincode::deserialize(&buf[..]).unwrap();

        println!("{:?}", decoded);

        buf.reverse();
        socket.send_to(buf, &src).unwrap();
    }
}
