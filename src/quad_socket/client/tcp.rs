use std::io::{ErrorKind, Write};
use std::net::ToSocketAddrs;

use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};

use crate::{error::Error, quad_socket::protocol::MessageReader};

pub struct TcpSocket {
    stream: TcpStream,
    rx: Receiver<Vec<u8>>,
}

impl TcpSocket {
    pub fn send(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        use std::io::Write;

        self.stream.write_all(&(data.len() as u32).to_be_bytes())?;
        self.stream.write_all(data)
    }

    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        /*        match self.rx.try_recv() {
            Ok(data) => Some(data),
            Err(error) => {
                if error == TryRecvError::Empty {
                    None
                } else {
                    // If the receiver is disconnected, we should close the socket
                    self.close();
                    None
                }
            }
        }*/
        self.rx.try_recv().ok()
    }

    pub fn close(&self) -> Result<(), std::io::Error> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }

    pub fn connected(&self) -> bool {
        let mut buf = [0; 1]; // A small buffer
        match self.stream.peek(&mut buf) {
            Ok(0) => true,                                        // Connection closed by peer
            Ok(_) => false,                                       // Data available, still connected
            Err(e) if e.kind() == ErrorKind::WouldBlock => false, // Would block means it's still open
            Err(_) => true, // Any other error means it's disconnected
        }
    }
}

impl TcpSocket {
    pub fn connect<A: ToSocketAddrs>(addr: A) -> Result<TcpSocket, Error> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?; // TODO do we need this?

        let (tx, rx) = mpsc::channel();

        std::thread::spawn({
            let mut stream = stream.try_clone().expect("Failed to clone tcp stream");
            move || {
                let mut messages = MessageReader::new();
                loop {
                    if let Ok(Some(message)) = messages.next(&mut stream) {
                        if tx.send(message).is_err() {
                            // This only happens if the channel is disconnected.
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                            break;
                        }
                    }
                }
            }
        });

        Ok(TcpSocket { stream, rx })
    }
}
