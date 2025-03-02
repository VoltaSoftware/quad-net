use crate::error::Error;
use crate::quad_socket::client::{IncomingSocketMessage, OutgoingSocketMessage};
use log::error;
use std::sync::mpsc::{Receiver, Sender};
use tungstenite::{connect, Bytes, Message};

pub struct WebSocket {
    rx: Receiver<IncomingSocketMessage>,
    tx: Sender<OutgoingSocketMessage>,
}

impl WebSocket {
    pub fn send(&mut self, data: &[u8]) {
        let _ = self.tx.send(OutgoingSocketMessage::Send(data.to_vec()));
    }

    pub fn try_recv(&mut self) -> Option<IncomingSocketMessage> {
        self.rx.try_recv().ok()
    }

    pub fn close(&mut self) {
        let _ = self.tx.send(OutgoingSocketMessage::Close);
    }
}

impl WebSocket {
    pub fn connect(addr: &'static str) -> WebSocket {
        let (incoming_sock_msg_tx, incoming_sock_msg_rx) = std::sync::mpsc::channel();
        let (outgoing_sock_msg_tx, outgoing_sock_msg_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let socket = connect(addr);
            if let Err(e) = socket {
                incoming_sock_msg_tx
                    .send(IncomingSocketMessage::Error(Error::from(e)))
                    .expect("Socket tx closed");
                return;
            }

            let (mut socket, response) = socket.unwrap();

            match socket.get_mut() {
                tungstenite::stream::MaybeTlsStream::Plain(stream) => stream.set_nonblocking(true),
                tungstenite::stream::MaybeTlsStream::Rustls(stream) => {
                    stream.get_mut().set_nonblocking(true)
                }
                e => unimplemented!("Unsupported stream type {:?}", e),
            }
            .expect("Failed to set nonblocking");

            incoming_sock_msg_tx
                .send(IncomingSocketMessage::Connected)
                .unwrap();

            'outer: loop {
                while let Some(msg) = outgoing_sock_msg_rx.try_recv().ok() {
                    match msg {
                        OutgoingSocketMessage::Close => {
                            let _ = incoming_sock_msg_tx.send(IncomingSocketMessage::Closed);

                            let _ = socket
                                .close(None)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
                            break 'outer;
                        }
                        OutgoingSocketMessage::Send(data) => {
                            let tokio_bytes = Bytes::from(data.to_vec());
                            let result = socket
                                .send(Message::Binary(tokio_bytes))
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

                            if let Err(e) = result {
                                let _ = incoming_sock_msg_tx
                                    .send(IncomingSocketMessage::Error(Error::from(e)));
                            }
                        }
                    }
                }

                if let Ok(msg) = socket.read() {
                    if let Message::Binary(data) = msg {
                        if let Err(err) = incoming_sock_msg_tx
                            .send(IncomingSocketMessage::PacketReceived(data.to_vec()))
                        {
                            error!("Failed to send incoming message: {:?}", err);
                            break;
                        }
                    }
                }

                if !socket.can_read() || !socket.can_write() {
                    let _ = incoming_sock_msg_tx.send(IncomingSocketMessage::Closed);
                    break 'outer;
                }
            }
        });

        //socket.get_mut().set_nodelay(true).map_err(|e| Error::from(e))?;

        WebSocket {
            rx: incoming_sock_msg_rx,
            tx: outgoing_sock_msg_tx,
        }
    }
}
