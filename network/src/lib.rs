use anyhow::*;
use lazy_static::lazy_static;
use log::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Packet {
    /// Sent from the client to the server on initial contact
    Login { username: [u8; 20] },

    /// Sent from the server to the client to create characters.
    CreateCharacter {
        /// Globally unique ID
        id: u32,
        username: [u8; 20],
        position: [f32; 3],

        /// If true, then this character is owned by the given connection
        is_owned: bool,
    },
}

pub struct Connection {
    uid: u32,
    //receiver: Receiver<Packet>,
    reliable: RefCell<TcpStream>,

    /// The buffer for bytes read from reliable
    buffer: RefCell<Vec<u8>>,
}

lazy_static! {
    static ref NEXT_UID: AtomicU32 = AtomicU32::new(0);
}

fn get_next_uid() -> u32 {
    NEXT_UID.fetch_add(1, Ordering::SeqCst)
}

impl Connection {
    /// An identifier which is unique amongst Connections for the process. Note that it
    /// likely will not match the uid for the Connection on the other end.
    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn connect() -> Result<Self> {
        let mut stream = TcpStream::connect("127.0.0.1:3419")?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            uid: get_next_uid(),
            reliable: RefCell::new(stream),
            buffer: RefCell::new(vec![]),
        })
    }

    pub fn send(&self, packet: &Packet) -> Result<()> {
        let encoded: Vec<u8> = bincode::serialize(packet)?;
        let size = encoded.len() as u16;
        self.reliable.borrow_mut().write(&size.to_le_bytes())?;
        self.reliable.borrow_mut().write(&encoded)?;
        Ok(())
    }

    pub fn update<F: FnMut(&Packet) -> Result<()>>(&self, mut cb: F) -> Result<()> {
        loop {
            let mut data = [0; 64];
            match self.reliable.borrow_mut().read(&mut data) {
                Ok(n) => {
                    for b in &data[..n] {
                        self.buffer.borrow_mut().push(*b);
                    }
                    if n == 0 {
                        return Ok(());
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(Error::new(e).context("Network"));
                }
            };

            if self.buffer.borrow().len() < 2 {
                continue;
            }
            let size =
                u16::from_le_bytes([self.buffer.borrow()[0], self.buffer.borrow()[1]]) as usize;
            if self.buffer.borrow().len() < 2 + size {
                continue;
            }

            // Parse this packet!
            let packet: Packet = bincode::deserialize(&self.buffer.borrow()[2..2 + size])
                .context("Decoding packet")?;
            cb(&packet);

            self.buffer.borrow_mut().drain(..2 + size);
        }
    }

    pub fn packets(&self) -> Result<Vec<Packet>> {
        let mut v: Vec<Packet> = vec![];
        self.update(|packet| {
            v.push(packet.clone());
            Ok(())
        })?;
        Ok(v)
    }
}

pub struct ConnectionListener {
    listener: TcpListener,
}

impl ConnectionListener {
    pub fn new() -> Result<Self> {
        let mut tcp_listener = TcpListener::bind("127.0.0.1:3419").unwrap();
        tcp_listener.set_nonblocking(true).unwrap();
        Ok(Self {
            listener: tcp_listener,
        })
    }

    pub fn update(&mut self) -> Result<Option<Connection>> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                info!("Connection received from {:?}", addr);
                stream.set_nonblocking(true)?;
                return Ok(Some(Connection {
                    uid: get_next_uid(),
                    reliable: RefCell::new(stream),
                    buffer: RefCell::new(vec![]),
                }));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(None);
            }
            Err(e) => {
                return Err(Error::new(e));
            }
        }
    }
}
