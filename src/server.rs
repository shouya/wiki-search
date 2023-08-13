mod handler;

use std::{net::SocketAddr, sync::Arc};

use axum::{
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{get, post},
  Extension, Router,
};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;

use crate::{
  search::Search,
  util::{Error, Result},
  wiki::Wiki,
};

pub struct Server {
  bind_addr: SocketAddr,
  search: Arc<RwLock<Search>>,
  wiki: Arc<Mutex<Wiki>>,
}

type SearchE = Extension<Arc<RwLock<Search>>>;
type WikiE = Extension<Arc<Mutex<Wiki>>>;

impl Server {
  pub fn new(bind_addr: SocketAddr, search: Search, wiki: Wiki) -> Self {
    let search = Arc::new(RwLock::new(search));
    let wiki = Arc::new(Mutex::new(wiki));

    Self {
      bind_addr,
      search,
      wiki,
    }
  }

  pub async fn run(self) -> Result<()> {
    let router = self.router();

    axum::Server::bind(&self.bind_addr)
      .serve(router.into_make_service())
      .await?;

    Ok(())
  }

  fn router(&self) -> Router {
    Router::new()
      .route("/", get(handler::index))
      .route("/search", get(handler::search))
      .route("/reindex", post(handler::reindex))
      // .route("/morelikethis", get(handler::morelikethis))
      .layer(Extension(self.search.clone()))
      .layer(Extension(self.wiki.clone()))
  }
}

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    warn!("Error: {:?}", self);
    StatusCode::BAD_REQUEST.into_response()
  }
}
