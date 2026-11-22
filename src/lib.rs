//! A click counter app, serving its counter API and its web frontend embedded
//! in the binary.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use include_dir::{Dir, File, include_dir};
use serde_json::{Value, json};

/// The built web frontend, embedded from where the build script builds it.
static WEB: Dir<'static> = include_dir!("$OUT_DIR/web");

/// The web frontend's `index.html`, served for paths without a matching file.
const INDEX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web/index.html"));

/// Creates the router that serves the counter API and the embedded web
/// frontend.
///
/// The API lives under `/api`, where `GET /api/counter` returns the count and
/// `POST /api/counter/increment` increments it, both as `{"count": n}`. The
/// count is kept in memory, starting from zero. Unknown paths under `/api` get
/// 404.
///
/// Requests for other paths without a matching file fall back to the web
/// frontend's `index.html`, so its client-side router handles them, including
/// unknown paths.
pub fn router() -> Router {
    let api = Router::new()
        .route("/counter", get(get_count))
        .route("/counter/increment", post(increment_count))
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(Arc::new(AtomicU64::new(0)));
    Router::new().nest("/api", api).fallback_service(get(serve))
}

/// Returns the current count.
async fn get_count(State(count): State<Arc<AtomicU64>>) -> Json<Value> {
    Json(json!({ "count": count.load(Ordering::Relaxed) }))
}

/// Increments the count and returns the new one.
async fn increment_count(State(count): State<Arc<AtomicU64>>) -> Json<Value> {
    Json(json!({ "count": count.fetch_add(1, Ordering::Relaxed) + 1 }))
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
    use http_body_util::BodyExt;
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

    fn json_response(body: &str) -> (StatusCode, String, Vec<u8>) {
        (
            StatusCode::OK,
            "application/json".to_owned(),
            body.as_bytes().to_vec(),
        )
    }

    #[tokio::test]
    async fn get_initial_count() {
        let response = send(&router(), Method::GET, "/api/counter").await;
        assert_eq!(response, json_response(r#"{"count":0}"#));
    }

    #[tokio::test]
    async fn increment_count_twice() {
        let app = router();
        let response = send(&app, Method::POST, "/api/counter/increment").await;
        assert_eq!(response, json_response(r#"{"count":1}"#));
        let response = send(&app, Method::POST, "/api/counter/increment").await;
        assert_eq!(response, json_response(r#"{"count":2}"#));
        let response = send(&app, Method::GET, "/api/counter").await;
        assert_eq!(response, json_response(r#"{"count":2}"#));
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
