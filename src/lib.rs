//! A click counter app, syncing its counter over a WebSocket to its web
//! frontend embedded in the binary.

mod counter;
mod live;

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
        .route("/ws", get(live::upgrade::<Counter>))
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
    use crate::live::testing::Client;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

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

    #[tokio::test]
    async fn serve_counter_over_websocket() {
        let mut client = Client::connect(router(), "/api/ws").await;
        client
            .send(&json!({"type": "subscribe", "key": "counter"}))
            .await;
        let sync = json!({"type": "sync", "key": "counter", "version": 0, "value": 0});
        assert_eq!(client.receive().await, sync);
        let call = json!({"type": "call", "id": 1, "method": "counter.increment"});
        client.send(&call).await;
        let sync = json!({"type": "sync", "key": "counter", "version": 1, "value": 1});
        assert_eq!(client.receive().await, sync);
        let result = json!({"type": "result", "id": 1, "value": null});
        assert_eq!(client.receive().await, result);
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
