//! Live sync between the app and its pages: calls that change state on the
//! server, and synced objects that the server owns and pushes whole to every
//! page subscribed to them.
//!
//! This module is written as if it were a separate library. The app
//! implements [`Service`] with its calls and synced objects, keeps their
//! values in [`Synced`], and serves it with [`upgrade`]. The protocol logic
//! for one client lives in a session that doesn't depend on the WebSocket, so
//! it's tested without one.

mod protocol;
mod service;
mod session;
mod synced;
#[cfg(test)]
pub mod testing;
mod websocket;

pub use service::{Error, Service};
pub use synced::{Synced, Updates};
pub use websocket::upgrade;
