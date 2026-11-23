//! Test helpers: a fake service, and a WebSocket client for a real server.

use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::{Error, Service, Synced, Updates};

/// A service with one synced object, `value`, which every call increments.
///
/// Each call waits once before replying with its params, or with an error if
/// its method is `fail`.
pub struct Fake {
    /// The synced object at `value`.
    value: Synced<u64>,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            value: Synced::new(0),
        }
    }
}

impl Service for Fake {
    async fn call(&self, method: &str, params: &Value) -> Result<Value, Error> {
        tokio::task::yield_now().await;
        if method == "fail" {
            Err(Error::new("failed", "the call failed"))
        } else {
            self.value.modify(|value| *value += 1);
            Ok(params.clone())
        }
    }

    fn subscribe(&self, key: &str) -> Option<Updates> {
        (key == "value").then(|| self.value.subscribe())
    }
}

/// A WebSocket client connected to a server running in the background.
pub struct Client {
    /// The client's end of the connection.
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Client {
    /// Starts a server running `router` and connects to it at `path`.
    pub async fn connect(router: Router, path: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(axum::serve(listener, router).into_future());
        let url = format!("ws://{address}{path}");
        let (stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        Self { stream }
    }

    /// Sends `message` as JSON.
    pub async fn send(&mut self, message: &Value) {
        let text = message.to_string();
        self.send_raw(Message::Text(text.into())).await;
    }

    /// Sends `message` as is.
    pub async fn send_raw(&mut self, message: Message) {
        self.stream.send(message).await.unwrap();
    }

    /// Receives the next message, parsed from JSON.
    pub async fn receive(&mut self) -> Value {
        let message = self.stream.next().await.unwrap().unwrap();
        serde_json::from_str(&message.into_text().unwrap()).unwrap()
    }

    /// Returns whether the server closes the connection instead of sending
    /// another message.
    pub async fn is_closed(&mut self) -> bool {
        matches!(
            self.stream.next().await,
            None | Some(Err(_) | Ok(Message::Close(_)))
        )
    }

    /// Closes the connection, waiting until the server has closed it too.
    pub async fn close(mut self) {
        self.stream.close(None).await.unwrap();
        while self.stream.next().await.is_some() {}
    }
}
