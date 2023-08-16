mod ui;

use axum::{
  extract::{Host, OriginalUri, WebSocketUpgrade},
  response::{Html, IntoResponse},
  routing::get,
  Extension, Router,
};

use super::{SearchRef, WikiRef};

#[derive(Clone, derive_more::Deref)]
pub struct LiveViewPool(dioxus_liveview::LiveViewPool);

pub fn router() -> Router {
  let view = dioxus_liveview::LiveViewPool::new();

  Router::new()
    .route("/", get(index))
    .route("/ws", get(websocket))
    .layer(Extension(LiveViewPool(view)))
}

pub async fn index(
  Host(host): Host,
  OriginalUri(uri): OriginalUri,
) -> impl IntoResponse {
  let mut base = format!("{host}{uri}");
  base.truncate(base.rfind('/').unwrap_or(base.len()));

  let glue = dioxus_liveview::interpreter_glue(&format!("ws://{base}/ws"));
  let bytes = StaticAsset::get("index.html").unwrap().data;
  let html = String::from_utf8_lossy(&bytes).replace("{liveview_glue}", &glue);

  Html(html)
}

pub async fn websocket(
  Extension(view): Extension<LiveViewPool>,
  Extension(wiki): Extension<WikiRef>,
  Extension(search): Extension<SearchRef>,
  ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| async move {
    let socket = dioxus_liveview::axum_socket(socket);
    let props = ui::AppProps {
      wiki,
      search,
      tag: Default::default(),
    };
    let _ = view.launch_with_props(socket, ui::App, props).await;
  })
}

#[derive(rust_embed::RustEmbed)]
#[folder = "static/"]
struct StaticAsset;
