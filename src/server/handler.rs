use axum::Json;
use serde::{Deserialize, Serialize};

use super::*;

pub use index::index;
pub use reindex::reindex;
pub use search::search;

mod index {
  // use super::*;

  pub async fn index() -> String {
    "Hello, World!".to_string()
  }
}

mod reindex {
  use super::*;

  pub async fn reindex(
    Extension(search): SearchE,
    Extension(wiki): WikiE,
  ) -> Result<()> {
    let pages = wiki.lock().await.list_pages().await?;
    search.write().await.reindex_pages(pages)?;
    Ok(())
  }
}

mod search {
  use crate::search::QueryOptions;

  use super::*;
  use axum::extract::Query;

  fn default_snippet_prefix() -> String {
    "<span class=\"term\">".into()
  }
  fn default_snippet_suffix() -> String {
    "</span>".into()
  }

  #[derive(Deserialize)]
  pub struct SnippetOptions {
    #[serde(default = "default_snippet_prefix", alias = "snippet_prefix")]
    prefix: String,
    #[serde(default = "default_snippet_suffix", alias = "snippet_suffix")]
    suffix: String,
  }

  #[derive(Deserialize)]
  pub struct SearchRequest {
    q: String,

    #[serde(flatten)]
    options: QueryOptions,

    #[serde(flatten)]
    snippet_options: SnippetOptions,
  }

  #[derive(Serialize)]
  struct SearchResult {
    title: String,
    text: String,
  }

  #[derive(Serialize)]
  pub struct SearchResponse {
    results: Vec<SearchResult>,
  }

  pub async fn search(
    Query(req): Query<SearchRequest>,
    Extension(search): SearchE,
  ) -> Result<Json<SearchResponse>> {
    let guard = search.read().await;
    let mut results = vec![];
    for mut entry in guard.query(&req.q, &req.options)? {
      entry.highlight(&req.snippet_options.prefix, &req.snippet_options.suffix);
      let title = entry.title;
      let text = entry.text;

      let result = SearchResult { title, text };
      results.push(result);
    }

    Ok(Json(SearchResponse { results }))
  }
}
