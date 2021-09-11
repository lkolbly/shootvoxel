use anyhow::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Serialize, Deserialize)]
pub enum Packet {
    Login { username: [u8; 20] },
}

pub struct Connection {
    reliable: TcpStream,

    /// The buffer for bytes read from reliable
    buffer: Vec<u8>,
}

impl Connection {
    pub fn connect() -> Result<Self> {
        let mut stream = TcpStream::connect("127.0.0.1:3419")?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            reliable: stream,
            buffer: vec![],
        })
    }

    pub fn send(&mut self, packet: &Packet) -> Result<()> {
        let encoded: Vec<u8> = bincode::serialize(packet)?;
        let size = encoded.len() as u16;
        self.reliable.write(&size.to_le_bytes())?;
        self.reliable.write(&encoded)?;
        Ok(())
    }

    pub fn update<F: FnMut(&Packet) -> Result<()>>(&mut self, mut cb: F) -> Result<()> {
        loop {
            let mut data = [0; 64];
            match self.reliable.read(&mut data) {
                Ok(n) => {
                    for b in &data[..n] {
                        self.buffer.push(*b);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(Error::new(e).context("Network"));
                }
            };

            if self.buffer.len() < 2 {
                continue;
            }
            let size = u16::from_le_bytes([self.buffer[0], self.buffer[1]]) as usize;
            if self.buffer.len() < 2 + size {
                continue;
            }

            // Parse this packet!
            let packet: Packet =
                bincode::deserialize(&self.buffer[2..2 + size]).context("Decoding packet")?;
            cb(&packet);

            self.buffer.drain(..2 + size);
        }
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
                return Ok(Some(Connection {
                    reliable: stream,
                    buffer: vec![],
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
