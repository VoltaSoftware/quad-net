//! Various network abstractions over web and desktop.

pub mod error;

pub mod http_request;
#[cfg(target_arch = "wasm32")]
mod js_object;
pub mod quad_socket;
pub mod web_socket;

#[cfg(target_arch = "wasm32")]
pub use js_object::{JsObject, JsObjectWeak};

#[unsafe(no_mangle)]
pub extern "C" fn quad_net_crate_version() -> u32 {
    1
}
