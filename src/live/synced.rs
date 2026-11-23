//! Values that the server owns and syncs whole to every subscribed client.

use std::pin::Pin;

use futures_util::{Stream, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

/// A value that the server owns and syncs whole to every subscribed client,
/// with a version that increases on every change.
pub struct Synced<T> {
    /// The channel holding the current value and its version.
    sender: watch::Sender<Versioned<T>>,
}

/// A value at one version.
#[derive(Clone)]
struct Versioned<T> {
    /// The version, increased on every change.
    version: u64,
    /// The value at this version.
    value: T,
}

impl<T: Clone + Serialize + Send + Sync + 'static> Synced<T> {
    /// Creates a synced object holding `value` at version 0.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            sender: watch::Sender::new(Versioned { version: 0, value }),
        }
    }

    /// Changes the value with `modify` and syncs it at the next version.
    pub fn modify(&self, modify: impl FnOnce(&mut T)) {
        self.sender.send_modify(|current| {
            current.version += 1;
            modify(&mut current.value);
        });
    }

    /// Returns the updates of this object, starting from its current value.
    ///
    /// A subscriber that falls behind skips straight to the latest value.
    pub fn subscribe(&self) -> Updates {
        Box::pin(
            WatchStream::new(self.sender.subscribe()).map(|current| Snapshot {
                version: current.version,
                value: serde_json::to_value(current.value)
                    .expect("synced value should serialize to JSON"),
            }),
        )
    }
}

/// The updates of a synced object, starting from its current value.
pub type Updates = Pin<Box<dyn Stream<Item = Snapshot> + Send>>;

/// A synced object's whole value at one version, serialized to JSON.
#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The version, increased on every change.
    pub version: u64,
    /// The value at this version.
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(version: u64, value: i32) -> Snapshot {
        Snapshot {
            version,
            value: json!(value),
        }
    }

    #[tokio::test]
    async fn update_from_current_value() {
        let synced = Synced::new(1);
        let mut updates = synced.subscribe();
        assert_eq!(updates.next().await, Some(snapshot(0, 1)));
        synced.modify(|value| *value += 1);
        assert_eq!(updates.next().await, Some(snapshot(1, 2)));
    }

    #[tokio::test]
    async fn skip_to_latest_value() {
        let synced = Synced::new(1);
        let mut updates = synced.subscribe();
        assert_eq!(updates.next().await, Some(snapshot(0, 1)));
        synced.modify(|value| *value += 1);
        synced.modify(|value| *value += 1);
        assert_eq!(updates.next().await, Some(snapshot(2, 3)));
    }
}
