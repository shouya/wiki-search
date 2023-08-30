mod fragment;
mod ui;

use axum::{response::IntoResponse, routing::get, Router};
use http::Uri;

pub fn router() -> Router {
  Router::new()
    .route("/", get(static_file))
    .route("/style.css", get(static_file))
    .route("/script.js", get(static_file))
    .nest("/frag", fragment::router())
}

pub async fn static_file(uri: Uri) -> impl IntoResponse {
  use axum::http::header;
  let mut path = uri.path().strip_prefix('/').unwrap_or(uri.path());
  if path.is_empty() {
    path = "index.html";
  }

  if let Some(file) = StaticAsset::get(path) {
    let mime_type = mime_guess::from_path(path).first_or_octet_stream();
    let header = [(header::CONTENT_TYPE, mime_type.as_ref())];
    (header, file.data).into_response()
  } else {
    axum::http::StatusCode::NOT_FOUND.into_response()
  }
}

#[derive(rust_embed::RustEmbed)]
#[folder = "static/"]
struct StaticAsset;
