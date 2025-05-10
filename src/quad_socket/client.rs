use std::net::ToSocketAddrs;

#[cfg(not(target_arch = "wasm32"))]
mod websocket;
#[cfg(target_arch = "wasm32")]
use crate::web_socket::js_web_socket as websocket;

use crate::error::Error;

pub struct QuadSocket {
    #[cfg(not(target_arch = "wasm32"))]
    tcp_socket: websocket::WebSocket,
    #[cfg(target_arch = "wasm32")]
    web_socket: websocket::WebSocket,
}

impl QuadSocket {
    pub fn send(&mut self, data: &[u8]) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.tcp_socket.send(data)
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.web_socket.send_bytes(data)
        }
    }

    pub fn close(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.tcp_socket.close()
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.web_socket.close()
        }
    }

    pub fn try_recv(&mut self) -> Option<IncomingSocketMessage> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.tcp_socket.try_recv()
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.web_socket.try_recv()
        }
    }
}

impl QuadSocket {
    pub fn connect(addr: impl Into<String>) -> QuadSocket {
        QuadSocket {
            #[cfg(not(target_arch = "wasm32"))]
            tcp_socket: websocket::WebSocket::connect(addr),
            #[cfg(target_arch = "wasm32")]
            web_socket: websocket::WebSocket::connect(addr),
        }
    }
}

pub enum OutgoingSocketMessage {
    Close,
    Send(Vec<u8>),
}

pub enum IncomingSocketMessage {
    Connected,
    PacketReceived(Vec<u8>),
    Error(Error),
    Closed,
}
