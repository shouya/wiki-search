use axum::{
  routing::{get, post},
  Router,
};

use super::handler;

pub fn router() -> Router {
  Router::new()
    .route("/search", get(handler::search))
    .route("/reindex", post(handler::reindex))
  // .route("/morelikethis", get(handler::morelikethis))
}
