use network::{Connection, ConnectionListener, Packet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

const MTU: usize = 508;

struct Player {
    id: u32,
    position: [f32; 3],
}

struct Game {
    players: HashMap<u32, Player>,
}

fn main() {
    let mut listener = ConnectionListener::new().unwrap();
    let mut connections = vec![];
    let mut game = Game {
        players: HashMap::new(),
    };

    loop {
        if let Some(cxn) = listener.update().unwrap() {
            connections.push(cxn);
        }

        for cxn in connections.iter_mut() {
            cxn.update(|packet| {
                println!("{:?}", packet);
                Ok(())
            })
            .unwrap();
        }
    }
}
