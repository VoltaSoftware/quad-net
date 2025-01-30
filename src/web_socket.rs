//! Websocket client. Works through native websockets on web and through ws-rs on the desktop.

#[cfg(target_arch = "wasm32")]
pub(crate) mod js_web_socket {
    use std::net::ToSocketAddrs;

    use sapp_jsutils::JsObject;

    use crate::error::Error;

    pub struct WebSocket;

    extern "C" {
        fn ws_connect(addr: JsObject);
        fn ws_send(buffer: JsObject);
        fn ws_try_recv() -> JsObject;
        fn ws_is_connected() -> i32;
    }

    impl WebSocket {
        pub fn send_text(&self, text: &str) {
            unsafe { ws_send(JsObject::string(text)) };
        }

        pub fn send_bytes(&self, data: &[u8]) {
            unsafe { ws_send(JsObject::buffer(data)) };
        }

        pub fn try_recv(&mut self) -> Option<Vec<u8>> {
            let data = unsafe { ws_try_recv() };
            if data.is_nil() == false {
                let is_text = data.field_u32("text") == 1;
                let mut buf = vec![];
                if is_text {
                    let mut s = String::new();
                    data.field("data").to_string(&mut s);
                    buf = s.into_bytes();
                } else {
                    data.field("data").to_byte_buffer(&mut buf);
                }
                return Some(buf);
            }
            None
        }

        pub fn connected(&self) -> bool {
            unsafe { ws_is_connected() == 1 }
        }

        pub fn connect<A: ToSocketAddrs + std::fmt::Display>(addr: A) -> Result<WebSocket, Error> {
            unsafe { ws_connect(JsObject::string(&format!("{}", addr))) };

            Ok(WebSocket)
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use js_web_socket::WebSocket;