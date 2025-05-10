//! Websocket client. Works through native websockets on web and through ws-rs on the desktop.

#[cfg(target_arch = "wasm32")]
pub(crate) mod js_web_socket {
    use crate::error::Error;
    use crate::quad_socket::client::IncomingSocketMessage;
    use sapp_jsutils::JsObject;

    pub struct WebSocket;

    const CONNECTED: u32 = 0;
    const PACKED_RECEIVED: u32 = 1;
    const SOCKET_ERROR: u32 = 2;
    const CLOSED: u32 = 3;

    extern "C" {
        fn ws_connect(addr: JsObject);
        fn ws_send(buffer: JsObject);
        fn ws_close();
        fn ws_try_recv() -> JsObject;
    }

    impl WebSocket {
        pub fn send_bytes(&self, data: &[u8]) {
            unsafe { ws_send(JsObject::buffer(data)) };
        }

        pub fn try_recv(&mut self) -> Option<IncomingSocketMessage> {
            let data = unsafe { ws_try_recv() };
            if data.is_nil() == false {
                let type_id = data.field_u32("type");
                return match type_id {
                    CONNECTED => Some(IncomingSocketMessage::Connected),
                    PACKED_RECEIVED => {
                        let mut buf = vec![];
                        data.field("data").to_byte_buffer(&mut buf);
                        Some(IncomingSocketMessage::PacketReceived(buf))
                    }
                    SOCKET_ERROR => {
                        let mut json_error = String::new();
                        data.field("data").to_string(&mut json_error);
                        Some(IncomingSocketMessage::Error(Error::from(
                            std::io::Error::new(std::io::ErrorKind::Other, json_error),
                        )))
                    }
                    CLOSED => Some(IncomingSocketMessage::Closed),
                    _ => None,
                };
            }
            None
        }

        pub fn connect(addr: impl Into<String>) -> WebSocket {
            unsafe { ws_connect(JsObject::string(&format!("{}", addr))) };
            WebSocket
        }

        pub fn close(&self) {
            unsafe { ws_close() };
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use js_web_socket::WebSocket;
