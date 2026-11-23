//! The messages that clients and the server exchange.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A message that a client sends.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Calls `method` with `params`, expecting a reply with the same `id`.
    Call {
        /// The ID that the reply carries.
        id: u64,
        /// The name of the method to call.
        method: String,
        /// The method's parameters, `null` if omitted.
        #[serde(default)]
        params: Value,
    },
    /// Subscribes to the synced object at `key`.
    Subscribe {
        /// The synced object's key.
        key: String,
    },
    /// Unsubscribes from the synced object at `key`.
    Unsubscribe {
        /// The synced object's key.
        key: String,
    },
}

/// A message that the server sends.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Replies to the call with `id` with its result.
    Result {
        /// The ID of the call.
        id: u64,
        /// The call's result.
        value: Value,
    },
    /// Replies to the call with `id` with an error.
    Error {
        /// The ID of the call.
        id: u64,
        /// A machine-readable code identifying the kind of error.
        code: &'static str,
        /// A human-readable description of the error.
        message: String,
    },
    /// Syncs the whole value of the synced object at `key`.
    Sync {
        /// The synced object's key.
        key: String,
        /// The version of the value, increased on every change.
        version: u64,
        /// The value at this version.
        value: Value,
    },
    /// Rejects the subscription to the synced object at `key`.
    SyncError {
        /// The synced object's key.
        key: String,
        /// A machine-readable code identifying the kind of error.
        code: &'static str,
        /// A human-readable description of the error.
        message: String,
    },
}
