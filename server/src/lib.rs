//! The server of a full-stack app, serving the app's built static files.

use axum::Router;
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};

/// Creates the router that serves the built app from `public_dir`.
///
/// Requests for paths without a matching file fall back to the app's
/// `index.html`, so the app's client-side router handles them, including
/// unknown paths. Without a `public_dir`, no static files are served.
pub fn router(public_dir: Option<&Path>) -> Router {
    let Some(public_dir) = public_dir else {
        return Router::new();
    };
    let index = ServeFile::new(public_dir.join("index.html"));
    Router::new().fallback_service(ServeDir::new(public_dir).fallback(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn public_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "index").unwrap();
        std::fs::write(dir.path().join("app.js"), "app").unwrap();
        dir
    }

    async fn get(router: Router, uri: &str) -> (StatusCode, String) {
        let request = Request::get(uri).body(Body::empty()).unwrap();
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn serve_existing_file() {
        let dir = public_dir();
        let response = get(router(Some(dir.path())), "/app.js").await;
        assert_eq!(response, (StatusCode::OK, "app".to_owned()));
    }

    #[tokio::test]
    async fn serve_index_on_root() {
        let dir = public_dir();
        let response = get(router(Some(dir.path())), "/").await;
        assert_eq!(response, (StatusCode::OK, "index".to_owned()));
    }

    #[tokio::test]
    async fn fall_back_to_index_on_unknown_path() {
        let dir = public_dir();
        let response = get(router(Some(dir.path())), "/does/not/exist").await;
        assert_eq!(response, (StatusCode::OK, "index".to_owned()));
    }

    #[tokio::test]
    async fn serve_nothing_without_public_dir() {
        let response = get(router(None), "/").await;
        assert_eq!(response.0, StatusCode::NOT_FOUND);
    }
}
