//! The protocol logic for one client, independent of how its messages are
//! transported.

use std::pin::Pin;
use std::sync::Arc;

use futures_util::{FutureExt, StreamExt};
use serde_json::Value;
use tokio_stream::StreamMap;

use super::protocol::{ClientMessage, ServerMessage};
use super::service::{Error, Service};
use super::synced::{Snapshot, Updates};

/// The future of a call's result, boxed so a session can serve any
/// [`Service`] through one [`DynService`].
type CallFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, Error>> + Send + 'a>>;

/// A [`Service`] with its calls' futures boxed, so the session code is
/// compiled once rather than once per service.
trait DynService: Send + Sync {
    /// Handles a call to `method` with `params`, returning its result.
    fn call<'a>(&'a self, method: &'a str, params: &'a Value) -> CallFuture<'a>;

    /// Returns the updates of the synced object at `key`, or `None` if there's
    /// no such object.
    fn subscribe(&self, key: &str) -> Option<Updates>;
}

impl<S: Service> DynService for S {
    fn call<'a>(&'a self, method: &'a str, params: &'a Value) -> CallFuture<'a> {
        Box::pin(Service::call(self, method, params))
    }

    fn subscribe(&self, key: &str) -> Option<Updates> {
        Service::subscribe(self, key)
    }
}

/// One client's session: the service it calls and the synced objects it's
/// subscribed to.
pub struct Session {
    /// The service that the client calls and subscribes to.
    service: Arc<dyn DynService>,
    /// The updates of each synced object the client is subscribed to.
    subscriptions: StreamMap<String, Updates>,
}

impl Session {
    /// Creates a session serving `service`, with no subscriptions.
    #[must_use]
    pub fn new<S: Service>(service: Arc<S>) -> Self {
        Self {
            service,
            subscriptions: StreamMap::new(),
        }
    }

    /// Handles a client's `message`, returning the messages to reply with.
    pub async fn receive(&mut self, message: ClientMessage) -> Vec<ServerMessage> {
        match message {
            ClientMessage::Call { id, method, params } => self.call(id, &method, &params).await,
            ClientMessage::Subscribe { key } => self.subscribe(key),
            ClientMessage::Unsubscribe { key } => self.unsubscribe(&key),
        }
    }

    /// Waits for the next value of a subscribed object, returning the message
    /// that syncs it, or `None` right away if there are no subscriptions.
    pub async fn next_sync(&mut self) -> Option<ServerMessage> {
        let (key, snapshot) = self.subscriptions.next().await?;
        Some(sync(key, snapshot))
    }

    /// Calls `method` with `params`, returning the call's reply with `id`.
    ///
    /// The reply comes after the syncs of the changes the call made, so the
    /// client already has the new state once the call resolves.
    async fn call(&mut self, id: u64, method: &str, params: &Value) -> Vec<ServerMessage> {
        let reply = self.service.call(method, params).await.map_or_else(
            |error| ServerMessage::Error {
                id,
                code: error.code,
                message: error.message,
            },
            |value| ServerMessage::Result { id, value },
        );
        let mut replies = self.pending_syncs();
        replies.push(reply);
        replies
    }

    /// Subscribes to the synced object at `key`, returning an error to reply
    /// with if there's no such object.
    ///
    /// The object's current value is synced by the next [`Self::next_sync`].
    fn subscribe(&mut self, key: String) -> Vec<ServerMessage> {
        let Some(updates) = self.service.subscribe(&key) else {
            return vec![ServerMessage::SyncError {
                message: format!("no synced object at {key}"),
                key,
                code: "unknown_key",
            }];
        };
        self.subscriptions.insert(key, updates);
        Vec::new()
    }

    /// Unsubscribes from the synced object at `key`, which needs no reply.
    fn unsubscribe(&mut self, key: &str) -> Vec<ServerMessage> {
        self.subscriptions.remove(key);
        Vec::new()
    }

    /// Returns the syncs of every subscribed object whose value is ready,
    /// without waiting for any.
    fn pending_syncs(&mut self) -> Vec<ServerMessage> {
        std::iter::from_fn(|| self.subscriptions.next().now_or_never().flatten())
            .map(|(key, snapshot)| sync(key, snapshot))
            .collect()
    }
}

/// Creates the message that syncs `snapshot` of the synced object at `key`.
fn sync(key: String, snapshot: Snapshot) -> ServerMessage {
    ServerMessage::Sync {
        key,
        version: snapshot.version,
        value: snapshot.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::testing::Fake;
    use serde_json::json;

    fn session() -> Session {
        Session::new(Arc::new(Fake::default()))
    }

    fn call(id: u64, method: &str) -> ClientMessage {
        ClientMessage::Call {
            id,
            method: method.to_owned(),
            params: json!([id]),
        }
    }

    fn subscribe(key: &str) -> ClientMessage {
        ClientMessage::Subscribe {
            key: key.to_owned(),
        }
    }

    fn value_sync(version: u64, value: u64) -> ServerMessage {
        ServerMessage::Sync {
            key: "value".to_owned(),
            version,
            value: json!(value),
        }
    }

    #[tokio::test]
    async fn reply_to_call_with_result() {
        let mut session = session();
        let replies = session.receive(call(1, "increment")).await;
        let result = ServerMessage::Result {
            id: 1,
            value: json!([1]),
        };
        assert_eq!(replies, vec![result]);
    }

    #[tokio::test]
    async fn reply_to_call_with_error() {
        let mut session = session();
        let replies = session.receive(call(1, "fail")).await;
        let error = ServerMessage::Error {
            id: 1,
            code: "failed",
            message: "the call failed".to_owned(),
        };
        assert_eq!(replies, vec![error]);
    }

    #[tokio::test]
    async fn sync_current_value_on_subscribe() {
        let mut session = session();
        assert_eq!(session.receive(subscribe("value")).await, Vec::new());
        assert_eq!(session.next_sync().await, Some(value_sync(0, 0)));
    }

    #[tokio::test]
    async fn sync_changes_before_call_result() {
        let mut session = session();
        session.receive(subscribe("value")).await;
        assert_eq!(session.next_sync().await, Some(value_sync(0, 0)));
        let replies = session.receive(call(1, "increment")).await;
        let result = ServerMessage::Result {
            id: 1,
            value: json!([1]),
        };
        assert_eq!(replies, vec![value_sync(1, 1), result]);
    }

    #[tokio::test]
    async fn reject_unknown_key() {
        let mut session = session();
        let error = ServerMessage::SyncError {
            key: "nope".to_owned(),
            code: "unknown_key",
            message: "no synced object at nope".to_owned(),
        };
        assert_eq!(session.receive(subscribe("nope")).await, vec![error]);
        assert_eq!(session.next_sync().await, None);
    }

    #[tokio::test]
    async fn stop_syncing_after_unsubscribe() {
        let mut session = session();
        session.receive(subscribe("value")).await;
        assert_eq!(session.next_sync().await, Some(value_sync(0, 0)));
        let unsubscribe = ClientMessage::Unsubscribe {
            key: "value".to_owned(),
        };
        assert_eq!(session.receive(unsubscribe).await, Vec::new());
        assert_eq!(session.next_sync().await, None);
    }
}
