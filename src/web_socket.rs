//! Websocket client. Works through native websockets on web and through ws-rs on the desktop.

#[cfg(target_arch = "wasm32")]
pub(crate) mod js_web_socket {
    use std::net::{SocketAddr, ToSocketAddrs};

    use sapp_jsutils::JsObject;

    use crate::error::Error;

    pub struct WebSocket;

    extern "C" {
        fn ws_connect(addr: JsObject);
        fn ws_send(buffer: JsObject) -> JsObject;
        fn ws_close();
        fn ws_try_recv() -> JsObject;
        fn ws_is_connected() -> i32;
    }

    impl WebSocket {
        pub fn send_bytes(&self, data: &[u8]) -> Result<(), std::io::Error> {
            let error_obj: JsObject = unsafe { ws_send(JsObject::buffer(data)) };

            if error_obj.is_undefined() {
                Ok(())
            } else {
                let mut error_message = String::new();
                error_obj.to_string(&mut error_message);

                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error_message,
                ))
            }
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

        pub fn connect(addr: SocketAddr) -> Result<WebSocket, Error> {
            unsafe { ws_connect(JsObject::string(&format!("ws://{}", addr))) };

            Ok(WebSocket)
        }

        pub fn close(&self) -> Result<(), std::io::Error> {
            unsafe {
                ws_close();
            }
            Ok(()) // TODO can websockets fail to close?
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use js_web_socket::WebSocket;
