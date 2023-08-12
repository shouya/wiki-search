use axum::Json;
use serde::{Deserialize, Serialize};

use super::*;

pub use index::index;
pub use search::search;

mod index {
  // use super::*;

  pub async fn index() -> String {
    "Hello, World!".to_string()
  }
}

mod search {
  use super::*;
  use axum::extract::Query;

  #[derive(Deserialize)]
  pub struct SearchRequest {
    q: String,
  }

  #[derive(Serialize)]
  struct SearchResult {
    id: String,
    title: String,
    description: String,
    url: String,
  }

  #[derive(Serialize)]
  pub struct SearchResponse {
    results: Vec<SearchResult>,
  }

  #[axum::debug_handler]
  pub async fn search(
    Query(_req): Query<SearchRequest>,
    Extension(_search): SearchE,
  ) -> Result<Json<SearchResponse>> {
    todo!()
  }
}
