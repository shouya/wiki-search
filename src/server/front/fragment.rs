use axum::{routing::post, Extension, Form, Router};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tantivy::DateTime;

use crate::{
  search::{PageMatchEntry, QueryOptions},
  server::SearchRef,
  util::Result,
};

pub fn router() -> Router {
  Router::new().route("/search", post(search))
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
  let q = if form.q.is_empty() { "*" } else { &form.q };

  let result = search.query(q, &options)?;
  let header = html! {
    div class="search-result-header" {
      "Query: " (form.q)
      " (" (result.remaining) " results left)"
      " (elapsed: " (format!("{:.2?}", result.elapsed)) ")"
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
    @if let Some(offset) = result.new_offset {
      hr;
      div hx-trigger="revealed" hx-post="frag/search" hx-include="#query-form" hx-swap="outerHTML" hx-vals={"{\"offset\":" (offset) "}"} {
        "Load next page"
      }
    }
  };

  let fragment = html! {
    (header)
    @for entry in result.entries {
      (render_entry(&entry))
    }
    (next_page)
  };

  Ok(fragment)
}
