//! A WebSocket protocol for calls, which change state on the server, and synced
//! objects, which the server owns and pushes to every client subscribed to
//! them.

use std::pin::Pin;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures_util::{FutureExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;
use tokio_stream::StreamMap;
use tokio_stream::wrappers::WatchStream;

/// The calls and synced objects that a WebSocket connection serves.
pub trait Service: Send + Sync + 'static {
    /// Handles a call to `method` with `params`, returning its result.
    fn call(
        &self,
        method: &str,
        params: &Value,
    ) -> impl Future<Output = Result<Value, Error>> + Send;

    /// Returns the updates of the synced object at `key`, or `None` if there's
    /// no such object.
    fn subscribe(&self, key: &str) -> Option<Updates>;
}

/// The future of a call's result, boxed so connections can serve any
/// [`Service`] through one [`DynService`].
type CallFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, Error>> + Send + 'a>>;

/// A [`Service`] with its calls' futures boxed, so the connection code is
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

/// An error that a call replies with.
pub struct Error {
    /// A machine-readable code identifying the kind of error.
    code: &'static str,
    /// A human-readable description of the error.
    message: String,
}

impl Error {
    /// Creates an error with a machine-readable `code` and a human-readable
    /// `message`.
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// A value that the server owns and syncs to every subscribed client, with a
/// version that increases on every change.
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

/// A synced object's value at one version, serialized to JSON.
pub struct Snapshot {
    /// The version, increased on every change.
    version: u64,
    /// The value at this version.
    value: Value,
}

/// A message that a client sends.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
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
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
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
    /// Syncs the value of the synced object at `key`.
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

/// Something that a connection has to handle next.
enum Event {
    /// A message from the client, or `None` or an error once it's gone.
    Message(Option<Result<Message, axum::Error>>),
    /// A new value of the synced object at the given key.
    Sync(String, Snapshot),
}

/// Upgrades a request to a WebSocket connection that serves `service`.
pub async fn upgrade<S: Service>(
    State(service): State<Arc<S>>,
    request: WebSocketUpgrade,
) -> Response {
    request.on_upgrade(move |socket| serve(socket, service))
}

/// Serves `service` over `socket` until the client disconnects or sends a
/// message that isn't valid.
async fn serve(mut socket: WebSocket, service: Arc<dyn DynService>) {
    let mut subscriptions: StreamMap<String, Updates> = StreamMap::new();
    loop {
        let event = tokio::select! {
            message = socket.recv() => Event::Message(message),
            Some((key, snapshot)) = subscriptions.next() => Event::Sync(key, snapshot),
        };
        let replies = match event {
            Event::Message(Some(Ok(Message::Text(text)))) => match serde_json::from_str(&text) {
                Ok(message) => handle(service.as_ref(), &mut subscriptions, message).await,
                Err(_) => break,
            },
            Event::Message(Some(Ok(_))) => continue,
            Event::Message(_) => break,
            Event::Sync(key, snapshot) => vec![sync(key, snapshot)],
        };
        for reply in &replies {
            socket.send(encode(reply)).await.ok();
        }
    }
}

/// Handles a client's `message`, returning the messages to reply with.
async fn handle(
    service: &dyn DynService,
    subscriptions: &mut StreamMap<String, Updates>,
    message: ClientMessage,
) -> Vec<ServerMessage> {
    match message {
        ClientMessage::Call { id, method, params } => {
            call(service, subscriptions, id, &method, &params).await
        }
        ClientMessage::Subscribe { key } => subscribe(service, subscriptions, key),
        ClientMessage::Unsubscribe { key } => unsubscribe(subscriptions, &key),
    }
}

/// Calls `method` with `params` on `service`, returning the call's reply
/// with `id`.
///
/// The reply comes after the syncs of the changes the call made, so the
/// client already has the new state once the call resolves.
async fn call(
    service: &dyn DynService,
    subscriptions: &mut StreamMap<String, Updates>,
    id: u64,
    method: &str,
    params: &Value,
) -> Vec<ServerMessage> {
    let reply = service.call(method, params).await.map_or_else(
        |error| ServerMessage::Error {
            id,
            code: error.code,
            message: error.message,
        },
        |value| ServerMessage::Result { id, value },
    );
    let mut replies = pending_syncs(subscriptions);
    replies.push(reply);
    replies
}

/// Subscribes to the synced object at `key` on `service`, returning an error
/// to reply with if there's no such object.
fn subscribe(
    service: &dyn DynService,
    subscriptions: &mut StreamMap<String, Updates>,
    key: String,
) -> Vec<ServerMessage> {
    let Some(updates) = service.subscribe(&key) else {
        return vec![ServerMessage::SyncError {
            message: format!("no synced object at {key}"),
            key,
            code: "unknown_key",
        }];
    };
    subscriptions.insert(key, updates);
    Vec::new()
}

/// Unsubscribes from the synced object at `key`, which needs no reply.
fn unsubscribe(subscriptions: &mut StreamMap<String, Updates>, key: &str) -> Vec<ServerMessage> {
    subscriptions.remove(key);
    Vec::new()
}

/// Returns the syncs of every subscribed object whose value is ready to be
/// sent, without waiting for any.
fn pending_syncs(subscriptions: &mut StreamMap<String, Updates>) -> Vec<ServerMessage> {
    std::iter::from_fn(|| subscriptions.next().now_or_never().flatten())
        .map(|(key, snapshot)| sync(key, snapshot))
        .collect()
}

/// Creates the message that syncs `snapshot` of the synced object at `key`.
fn sync(key: String, snapshot: Snapshot) -> ServerMessage {
    ServerMessage::Sync {
        key,
        version: snapshot.version,
        value: snapshot.value,
    }
}

/// Encodes `message` as a WebSocket text message.
fn encode(message: &ServerMessage) -> Message {
    let text = serde_json::to_string(message).expect("server message should serialize to JSON");
    Message::Text(text.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use futures_util::SinkExt;
    use serde_json::json;
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::tungstenite;
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

    /// A service whose calls wait once before echoing their params, and
    /// which has no synced objects.
    struct Echo;

    impl Service for Echo {
        async fn call(&self, _method: &str, params: &Value) -> Result<Value, Error> {
            tokio::task::yield_now().await;
            Ok(params.clone())
        }

        fn subscribe(&self, _key: &str) -> Option<Updates> {
            None
        }
    }

    async fn connect() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new()
            .route("/", get(upgrade::<Echo>))
            .with_state(Arc::new(Echo));
        tokio::spawn(axum::serve(listener, router).into_future());
        let (client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/"))
            .await
            .unwrap();
        client
    }

    #[tokio::test]
    async fn reply_to_call_that_waits() {
        let mut client = connect().await;
        let call = json!({"type": "call", "id": 1, "method": "echo", "params": [1, 2]});
        let text = call.to_string();
        client
            .send(tungstenite::Message::Text(text.into()))
            .await
            .unwrap();
        let message = client.next().await.unwrap().unwrap();
        let reply: Value = serde_json::from_str(&message.into_text().unwrap()).unwrap();
        assert_eq!(reply, json!({"type": "result", "id": 1, "value": [1, 2]}));
    }

    #[tokio::test]
    async fn stop_serving_on_client_close() {
        let mut client = connect().await;
        client.close(None).await.unwrap();
        while client.next().await.is_some() {}
    }
}
