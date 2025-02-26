use crate::error::Error;
use tungstenite::{connect, Bytes, Message};

pub struct WebSocket {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
}

impl WebSocket {
    pub fn send(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        let tokio_bytes = Bytes::from(data.to_vec());
        self.socket
            .send(Message::Binary(tokio_bytes))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        if let Ok(msg) = self.socket.read() {
            if let Message::Binary(data) = msg {
                return Some(data.to_vec());
            }
        }
        None
    }

    pub fn close(&mut self) -> Result<(), std::io::Error> {
        self.socket
            .close(None)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn connected(&self) -> bool {
        self.socket.can_read() && self.socket.can_write()
    }
}

impl WebSocket {
    pub fn connect(addr: &str) -> Result<WebSocket, Error> {
        let socket = connect(addr);
        if let Err(e) = socket {
            return Err(Error::from(e));
        }

        let (mut socket, response) = socket.unwrap();

        match socket.get_mut() {
            tungstenite::stream::MaybeTlsStream::Plain(stream) => stream.set_nonblocking(true),
            tungstenite::stream::MaybeTlsStream::Rustls(stream) => {
                stream.get_mut().set_nonblocking(true)
            }
            e => unimplemented!("Unsupported stream type {:?}", e),
        }?;

        //socket.get_mut().set_nodelay(true).map_err(|e| Error::from(e))?;

        Ok(WebSocket { socket })
    }
}
