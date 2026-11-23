//! The WebSocket transport, carrying a session's messages as JSON text
//! messages.

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;

use super::protocol::ServerMessage;
use super::service::Service;
use super::session::Session;

/// Something that a connection has to handle next.
enum Event {
    /// A message from the client, or `None` or an error once it's gone.
    Message(Option<Result<Message, axum::Error>>),
    /// A sync of a subscribed object's new value.
    Sync(ServerMessage),
}

/// Upgrades a request to a WebSocket connection that serves `service`.
pub async fn upgrade<S: Service>(
    State(service): State<Arc<S>>,
    request: WebSocketUpgrade,
) -> Response {
    request.on_upgrade(move |socket| serve(socket, Session::new(service)))
}

/// Carries `session`'s messages over `socket` until the client disconnects or
/// sends a message that isn't valid.
async fn serve(mut socket: WebSocket, mut session: Session) {
    loop {
        let event = tokio::select! {
            message = socket.recv() => Event::Message(message),
            Some(sync) = session.next_sync() => Event::Sync(sync),
        };
        let replies = match event {
            Event::Message(Some(Ok(Message::Text(text)))) => match serde_json::from_str(&text) {
                Ok(message) => session.receive(message).await,
                Err(_) => break,
            },
            Event::Message(Some(Ok(_))) => continue,
            Event::Message(_) => break,
            Event::Sync(sync) => vec![sync],
        };
        for reply in &replies {
            socket.send(encode(reply)).await.ok();
        }
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
    use crate::live::testing::{Client, Fake};
    use axum::Router;
    use axum::routing::get;
    use serde_json::json;
    use tokio_tungstenite::tungstenite;

    async fn connect() -> Client {
        let router = Router::new()
            .route("/", get(upgrade::<Fake>))
            .with_state(Arc::new(Fake::default()));
        Client::connect(router, "/").await
    }

    #[tokio::test]
    async fn exchange_messages_as_json() {
        let mut client = connect().await;
        client
            .send(&json!({"type": "subscribe", "key": "value"}))
            .await;
        let sync = json!({"type": "sync", "key": "value", "version": 0, "value": 0});
        assert_eq!(client.receive().await, sync);
        let call = json!({"type": "call", "id": 1, "method": "increment", "params": [1]});
        client.send(&call).await;
        let sync = json!({"type": "sync", "key": "value", "version": 1, "value": 1});
        assert_eq!(client.receive().await, sync);
        let result = json!({"type": "result", "id": 1, "value": [1]});
        assert_eq!(client.receive().await, result);
    }

    #[tokio::test]
    async fn ignore_non_text_message() {
        let mut client = connect().await;
        let binary = tungstenite::Message::Binary(vec![1, 2, 3].into());
        client.send_raw(binary).await;
        client
            .send(&json!({"type": "call", "id": 1, "method": "fail"}))
            .await;
        let error = json!({
            "type": "error",
            "id": 1,
            "code": "failed",
            "message": "the call failed",
        });
        assert_eq!(client.receive().await, error);
    }

    #[tokio::test]
    async fn disconnect_on_invalid_message() {
        let mut client = connect().await;
        client
            .send_raw(tungstenite::Message::Text("nope".into()))
            .await;
        assert!(client.is_closed().await);
    }

    #[tokio::test]
    async fn stop_serving_on_client_close() {
        connect().await.close().await;
    }
}
