use axum::{
  routing::{get, post},
  Extension, Form, Router,
};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tantivy::DateTime;

use crate::{
  search::{PageMatchEntry, PageMatchResult, QueryOptions},
  server::{SearchRef, WikiRef},
  util::{Error, Result},
};

pub fn router() -> Router {
  Router::new()
    .route("/search", post(search))
    .route("/reindex", post(reindex))
    .route("/index", get(index_info))
}

#[derive(Deserialize)]
struct SearchQuery {
  q: String,
  #[serde(deserialize_with = "crate::util::deserialize_date")]
  date_before: Option<DateTime>,
  #[serde(deserialize_with = "crate::util::deserialize_date")]
  date_after: Option<DateTime>,
  offset: Option<usize>,
}

#[axum::debug_handler]
async fn search(
  Extension(search): Extension<SearchRef>,
  Form(form): Form<SearchQuery>,
) -> Result<Markup> {
  let search = search.read().await;

  let options = QueryOptions {
    offset: form.offset.unwrap_or(0),
    snippet_length: 400,
    date_before: form.date_before,
    date_after: form.date_after,
    ..Default::default()
  };
  let q = if form.q.trim().is_empty() {
    "*"
  } else {
    &form.q
  };

  let fragment = match search.query(q, &options) {
    Ok(result) => render_result(result),
    Err(Error::InvalidQuery(e)) => {
      html! {
        (format!("{:?}", e))
      }
    }
    Err(e) => {
      html! {
        (format!("{:?}", e))
      }
    }
  };

  Ok(fragment)
}

fn render_result(result: PageMatchResult) -> Markup {
  let header = html! {
    div class="search-result-header" {
      (result.remaining) " results left "
      "(elapsed: " (format!("{:.2?}", result.elapsed)) ")"
    }
  };

  let render_entry = |entry: &PageMatchEntry| {
    let title = entry.title.highlight("<b>", "</b>");
    let text = entry.text.highlight("<b>", "</b>");

    html! {
      div {
        h3 style="font-weight: normal;" {
          a href=(entry.url) {
            (PreEscaped(title))
          }
        }
        p style="max-width: 40vw;" {
          (PreEscaped(text))
        }
      }
    }
  };

  let next_page = html! {
    hr;
    @match result.new_offset {
      Some(offset) => {
        div hx-trigger="revealed" hx-post="frag/search"
        hx-include="#query-form" hx-swap="outerHTML"
        hx-vals={"{\"offset\":" (offset) "}"} {
          "Load next page"
        }
      }
      None => {div { "End of results" }}
    }
  };

  html! {
    (header)
    @for entry in result.entries {
      (render_entry(&entry))
    }
    (next_page)
  }
}

async fn reindex(
  Extension(search): Extension<SearchRef>,
  Extension(wiki): Extension<WikiRef>,
) -> Result<Markup> {
  let start = std::time::Instant::now();
  let pages = wiki.lock().await.list_pages().await?;
  search.write().await.reindex_pages(pages)?;
  let page_count = search.read().await.page_count()?;

  let fragment = html! {
    "Indexed " (page_count) " pages "
    "in " (format!("{:.2?}", start.elapsed()))
  };

  Ok(fragment)
}

async fn index_info(Extension(search): Extension<SearchRef>) -> Result<Markup> {
  let page_count = search.read().await.page_count()?;
  let fragment = html! {
    "Indexed " (page_count) " pages"
  };

  Ok(fragment)
}
