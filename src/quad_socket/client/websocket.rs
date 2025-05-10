use crate::error::Error;
use crate::quad_socket::client::{IncomingSocketMessage, OutgoingSocketMessage};
use log::error;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
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
    pub fn connect(addr: impl Into<String>) -> WebSocket {
        let (incoming_sock_msg_tx, incoming_sock_msg_rx) = std::sync::mpsc::channel();
        let (outgoing_sock_msg_tx, outgoing_sock_msg_rx) = std::sync::mpsc::channel();

        let addr = addr.into();
        std::thread::spawn(move || {
            let socket = connect(addr);
            if let Err(e) = socket {
                incoming_sock_msg_tx
                    .send(IncomingSocketMessage::Error(Error::from(e)))
                    .expect("Socket tx closed");
                return;
            }

            let (mut websocket_out, response) = socket.unwrap();

            match websocket_out.get_mut() {
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

                            let _ = websocket_out
                                .close(None)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
                            break 'outer;
                        }
                        OutgoingSocketMessage::Send(data) => {
                            let result = websocket_out
                                .send(Message::Binary(data.into()))
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

                            if let Err(e) = result {
                                let _ = incoming_sock_msg_tx
                                    .send(IncomingSocketMessage::Error(Error::from(e)));
                            }
                        }
                    }
                }

                if let Ok(msg) = websocket_out.read() {
                    if let Message::Binary(data) = msg {
                        if let Err(err) = incoming_sock_msg_tx
                            .send(IncomingSocketMessage::PacketReceived(data.into()))
                        {
                            error!("Failed to send incoming message: {:?}", err);
                            break;
                        }
                    }
                } else {
                    let fps = 30;
                    let frame_time = Duration::from_millis(1000 / fps);
                    std::thread::sleep(frame_time);
                }

                if !websocket_out.can_read() || !websocket_out.can_write() {
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
