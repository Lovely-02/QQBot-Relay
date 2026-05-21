use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/web/"]
struct WebAssets;

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn serve(path: &str) -> Response {
    let asset = WebAssets::get(path).or_else(|| WebAssets::get("index.html"));

    match asset {
        Some(content) => {
            let ct = if path.contains('.') {
                content_type(path)
            } else {
                "text/html; charset=utf-8"
            };
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, ct)
                .header(header::CACHE_CONTROL, "no-cache, must-revalidate")
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

pub async fn serve_web() -> impl IntoResponse {
    serve("index.html")
}

pub async fn serve_web_rest(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    serve(path)
}
