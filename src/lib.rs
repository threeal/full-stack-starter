//! A click counter app, serving its web frontend embedded in the binary.

use axum::Router;
use axum::http::{Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use include_dir::{Dir, File, include_dir};

/// The built web frontend, embedded from where the build script builds it.
static WEB: Dir<'static> = include_dir!("$OUT_DIR/web");

/// The web frontend's `index.html`, served for paths without a matching file.
const INDEX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web/index.html"));

/// Creates the router that serves the embedded web frontend.
///
/// Requests for paths without a matching file fall back to the web
/// frontend's `index.html`, so its client-side router handles them, including
/// unknown paths.
pub fn router() -> Router {
    Router::new().fallback_service(get(serve))
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
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn get(uri: &str) -> (StatusCode, String, Vec<u8>) {
        let request = Request::get(uri).body(Body::empty()).unwrap();
        let response = router().oneshot(request).await.unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map_or_else(String::new, |value| value.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, content_type, body.to_vec())
    }

    #[tokio::test]
    async fn serve_existing_file() {
        let response = get("/react.svg").await;
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
        let response = get("/").await;
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
        let response = get("/does/not/exist").await;
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
        let request = Request::post("/").body(Body::empty()).unwrap();
        let response = router().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
