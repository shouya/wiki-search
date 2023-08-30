use axum::{routing::get, Router};
use maud::{html, Markup};

pub fn router() -> Router {
  Router::new()
    .route("/query-bar", get(query_bar))
    .route("/result", get(search_result))
}

async fn query_bar() -> Markup {
  html! {
    div #query-bar {
      "query-bar"
    }
  }
}

async fn search_result() -> Markup {
  html! {
    div #search-result {
      "loaded"
    }
  }
}
