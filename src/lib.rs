//! A click counter app, syncing its counter over a WebSocket to its web
//! frontend embedded in the binary.

mod counter;
mod socket;

use std::sync::Arc;

use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use include_dir::{Dir, File, include_dir};

use crate::counter::Counter;

/// The built web frontend, embedded from where the build script builds it.
static WEB: Dir<'static> = include_dir!("$OUT_DIR/web");

/// The web frontend's `index.html`, served for paths without a matching file.
const INDEX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web/index.html"));

/// Creates the router that serves the app's WebSocket and the embedded web
/// frontend.
///
/// The WebSocket at `/api/ws` serves the `counter.increment` call and the
/// `counter` synced object, kept in memory and starting from zero. Other paths
/// under `/api` get 404.
///
/// Requests for other paths without a matching file fall back to the web
/// frontend's `index.html`, so its client-side router handles them, including
/// unknown paths.
pub fn router() -> Router {
    let api = Router::new()
        .route("/ws", get(socket::upgrade::<Counter>))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(Arc::new(Counter::default()));
    Router::new().nest("/api", api).fallback_service(get(serve))
}

/// Serves the embedded file at the request's path, or `index.html` if there's
/// none.
async fn serve(uri: Uri) -> Response {
    WEB.get_file(uri.path().trim_start_matches('/'))
        .map_or_else(|| Html(INDEX).into_response(), file_response)
}

/// Creates a response with `file`'s contents and the content type guessed
/// from its path.
fn file_response(file: &'static File<'static>) -> Response {
    let mime = mime_guess::from_path(file.path()).first_or_octet_stream();
    ([(header::CONTENT_TYPE, mime.as_ref())], file.contents()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use futures_util::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::tungstenite;
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
    use tower::ServiceExt;

    type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

    async fn send(app: &Router, method: Method, uri: &str) -> (StatusCode, String, Vec<u8>) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map_or_else(String::new, |value| value.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, content_type, body.to_vec())
    }

    async fn start() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(axum::serve(listener, router()).into_future());
        address
    }

    async fn connect(address: SocketAddr) -> Client {
        let (client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/api/ws"))
            .await
            .unwrap();
        client
    }

    async fn write(client: &mut Client, message: Value) {
        let text = message.to_string();
        client
            .send(tungstenite::Message::Text(text.into()))
            .await
            .unwrap();
    }

    async fn read(client: &mut Client) -> Value {
        let message = client.next().await.unwrap().unwrap();
        serde_json::from_str(&message.into_text().unwrap()).unwrap()
    }

    fn counter_sync(version: u64, count: u64) -> Value {
        json!({"type": "sync", "key": "counter", "version": version, "value": count})
    }

    #[tokio::test]
    async fn sync_counter_on_subscribe() {
        let mut client = connect(start().await).await;
        write(&mut client, json!({"type": "subscribe", "key": "counter"})).await;
        assert_eq!(read(&mut client).await, counter_sync(0, 0));
    }

    #[tokio::test]
    async fn sync_increment_before_its_result() {
        let mut client = connect(start().await).await;
        write(&mut client, json!({"type": "subscribe", "key": "counter"})).await;
        assert_eq!(read(&mut client).await, counter_sync(0, 0));
        let call = json!({"type": "call", "id": 1, "method": "counter.increment"});
        write(&mut client, call).await;
        assert_eq!(read(&mut client).await, counter_sync(1, 1));
        let result = json!({"type": "result", "id": 1, "value": null});
        assert_eq!(read(&mut client).await, result);
    }

    #[tokio::test]
    async fn sync_increment_to_other_clients() {
        let address = start().await;
        let mut caller = connect(address).await;
        let mut watcher = connect(address).await;
        write(&mut watcher, json!({"type": "subscribe", "key": "counter"})).await;
        assert_eq!(read(&mut watcher).await, counter_sync(0, 0));
        let call = json!({"type": "call", "id": 1, "method": "counter.increment"});
        write(&mut caller, call).await;
        assert_eq!(read(&mut watcher).await, counter_sync(1, 1));
    }

    #[tokio::test]
    async fn stop_syncing_after_unsubscribe() {
        let mut client = connect(start().await).await;
        write(&mut client, json!({"type": "subscribe", "key": "counter"})).await;
        assert_eq!(read(&mut client).await, counter_sync(0, 0));
        write(
            &mut client,
            json!({"type": "unsubscribe", "key": "counter"}),
        )
        .await;
        let call = json!({"type": "call", "id": 1, "method": "counter.increment"});
        write(&mut client, call).await;
        let result = json!({"type": "result", "id": 1, "value": null});
        assert_eq!(read(&mut client).await, result);
    }

    #[tokio::test]
    async fn reject_unknown_method() {
        let mut client = connect(start().await).await;
        write(
            &mut client,
            json!({"type": "call", "id": 1, "method": "nope"}),
        )
        .await;
        let error = json!({
            "type": "error",
            "id": 1,
            "code": "unknown_method",
            "message": "no method named nope",
        });
        assert_eq!(read(&mut client).await, error);
    }

    #[tokio::test]
    async fn reject_unknown_key() {
        let mut client = connect(start().await).await;
        write(&mut client, json!({"type": "subscribe", "key": "nope"})).await;
        let error = json!({
            "type": "sync_error",
            "key": "nope",
            "code": "unknown_key",
            "message": "no synced object at nope",
        });
        assert_eq!(read(&mut client).await, error);
    }

    #[tokio::test]
    async fn ignore_non_text_message() {
        let mut client = connect(start().await).await;
        let binary = tungstenite::Message::Binary(vec![1, 2, 3].into());
        client.send(binary).await.unwrap();
        let call = json!({"type": "call", "id": 1, "method": "counter.increment"});
        write(&mut client, call).await;
        let result = json!({"type": "result", "id": 1, "value": null});
        assert_eq!(read(&mut client).await, result);
    }

    #[tokio::test]
    async fn disconnect_on_invalid_message() {
        let mut client = connect(start().await).await;
        let invalid = tungstenite::Message::Text("nope".into());
        client.send(invalid).await.unwrap();
        let next = client.next().await;
        assert!(matches!(
            next,
            None | Some(Err(_) | Ok(tungstenite::Message::Close(_)))
        ));
    }

    #[tokio::test]
    async fn reject_unknown_api_path() {
        let (status, _, _) = send(&router(), Method::GET, "/api/does/not/exist").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn serve_existing_file() {
        let response = send(&router(), Method::GET, "/react.svg").await;
        let file = WEB.get_file("react.svg").unwrap();
        assert_eq!(
            response,
            (
                StatusCode::OK,
                "image/svg+xml".to_owned(),
                file.contents().to_vec()
            )
        );
    }

    #[tokio::test]
    async fn serve_index_on_root() {
        let response = send(&router(), Method::GET, "/").await;
        assert_eq!(
            response,
            (
                StatusCode::OK,
                "text/html; charset=utf-8".to_owned(),
                INDEX.to_vec()
            )
        );
    }

    #[tokio::test]
    async fn fall_back_to_index_on_unknown_path() {
        let response = send(&router(), Method::GET, "/does/not/exist").await;
        assert_eq!(
            response,
            (
                StatusCode::OK,
                "text/html; charset=utf-8".to_owned(),
                INDEX.to_vec()
            )
        );
    }

    #[tokio::test]
    async fn reject_non_get_request() {
        let (status, _, _) = send(&router(), Method::POST, "/").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }
}
