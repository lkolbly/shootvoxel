use log::*;
use network::{ConnectionListener, Packet};
use rand::prelude::*;
use std::collections::HashMap;

struct Player {
    id: u32,
    position: [f32; 3],
}

struct Game {
    players: HashMap<u32, Player>,
}

fn main() {
    simple_logging::log_to_stderr(log::LevelFilter::Trace);

    let mut listener = ConnectionListener::new().unwrap();
    let mut connections = vec![];
    let mut game = Game {
        players: HashMap::new(),
    };
    let mut next_id = 0;
    let mut rng = rand::thread_rng();

    loop {
        if let Some(cxn) = listener.update().unwrap() {
            connections.push(cxn);
        }

        let mut updates = vec![];

        for cxn in connections.iter_mut() {
            cxn.update(|packet| {
                debug!("Received packet {:?}", packet);
                match packet {
                    Packet::Login { username } => {
                        // Send them the current game state
                        for (_, player) in game.players.iter() {
                            cxn.send(&Packet::CreateCharacter {
                                id: player.id,
                                username: [6; 20],
                                position: player.position.clone(),
                                is_owned: false,
                            })
                            .unwrap();
                        }

                        // Create the player
                        let x: f32 = rng.gen();
                        let z: f32 = rng.gen();
                        let player = Player {
                            id: next_id,
                            position: [x * 5.0, 0.0, z * 5.0],
                        };
                        cxn.send(&Packet::CreateCharacter {
                            id: player.id,
                            username: username.clone(),
                            position: player.position.clone(),
                            is_owned: true,
                        })
                        .unwrap();
                        updates.push((
                            cxn.uid(),
                            Packet::CreateCharacter {
                                id: player.id,
                                username: username.clone(),
                                position: player.position.clone(),
                                is_owned: false,
                            },
                        ));
                        game.players.insert(next_id, player);
                        next_id += 1;
                    }
                    Packet::CreateCharacter { .. } => {
                        panic!("Impossible packet!");
                    }
                }
                Ok(())
            })
            .unwrap();
        }

        for (exclusion, update) in updates.iter_mut() {
            for cxn in connections.iter() {
                if cxn.uid() != *exclusion {
                    cxn.send(&update).unwrap();
                }
            }
        }
    }
}
