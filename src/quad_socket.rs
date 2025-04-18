//! Client and server abstraction to make a socket-like connection
//! on both desktop and web.
//!
//! Works through TCP on the desktop and through WebSocket on web.
//! Server will be capable to receive connections with both TCP and WebSocket
//! and QuadSocket client will automatically use the only web tech available on
//! the current platform

pub mod client;
