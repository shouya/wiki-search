#![allow(non_snake_case)]

use std::marker::PhantomData;

use dioxus::{
  core::IntoDynNode,
  prelude::{
    dioxus_elements, fc_to_builder, inline_props, render, rsx, to_owned,
    use_future, use_shared_state, use_shared_state_provider, use_state,
    Element, GlobalAttributes, Props, Scope, UseState,
  },
};
use tantivy::DateTime;

use crate::{
  search::{PageMatchEntry, QueryOptions},
  server::{SearchRef, WikiRef},
};

#[derive(Clone, Props)]
pub struct AppProps<'a> {
  pub wiki: WikiRef,
  pub search: SearchRef,
  pub tag: PhantomData<&'a ()>,
}

pub fn App<'a>(cx: Scope<'a, AppProps<'a>>) -> Element {
  use_shared_state_provider(cx, || cx.props.wiki.clone());
  use_shared_state_provider(cx, || cx.props.search.clone());

  let query = use_state(cx, String::new);
  let date_before = use_state(cx, || None);
  let date_after = use_state(cx, || None);

  render! {
    div {
      QueryBar {
        query: query.clone(),
        date_before: date_before.clone(),
        date_after: date_after.clone()
      }
      SearchResult {
        query: query.clone(),
        date_before: date_before.clone(),
        date_after: date_after.clone()
      }
    }
  }
}

#[inline_props]
fn QueryBar(
  cx: Scope,
  query: UseState<String>,
  date_before: UseState<Option<DateTime>>,
  date_after: UseState<Option<DateTime>>,
) -> Element {
  use chrono::NaiveDate;

  let set_date = |state: UseState<Option<DateTime>>, s: &str| {
    if s.is_empty() {
      state.set(None);
      return;
    }

    let Some(naive_date) = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok() else {
      return;
    };
    let date_time = naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let date_time = DateTime::from_timestamp_secs(date_time.timestamp());
    state.set(Some(date_time));
  };

  render! {
    div {
      div {
        label {
          "query: "
          input {
            placeholder: "Enter query here...",
            oninput: move |e| { query.set(e.value.clone()) }
          }
        }
      }
      div {
        label {
          "before: "
          input {
            placeholder: "2023-01-02",
            oninput: move |e| { set_date(date_before.clone(), &e.value) }
          }
        }
        label {
          "after: "
          input {
            placeholder: "2023-01-02",
            oninput: move |e| { set_date(date_after.clone(), &e.value) }
          }
        }
      }
    }
  }
}

struct Rendered<T>(pub T);

impl<'a, 'b> IntoDynNode<'a> for Rendered<&'b Vec<PageMatchEntry>> {
  fn into_vnode(
    self,
    cx: &'a dioxus::prelude::ScopeState,
  ) -> dioxus::core::DynamicNode<'a> {
    rsx! {
      for entry in self.0.iter() {
        {
          let title = entry.title.highlight("<b>", "</b>");
          let text = entry.text.highlight("<b>", "</b>");

          rsx! {
            div {
              h3 {
                style: "font-weight: normal;",
                a {
                  href: "{entry.url}",
                  dangerous_inner_html: "{title}"
                }
              }
              p {
                style: "max-width: 40vw;",
                dangerous_inner_html: "{text}"
              }
            }
          }
        }
      }
    }
    .into_vnode(cx)
  }
}

#[inline_props]
fn SearchResult(
  cx: Scope,
  query: UseState<String>,
  date_before: UseState<Option<DateTime>>,
  date_after: UseState<Option<DateTime>>,
) -> Element {
  let search = use_shared_state::<SearchRef>(cx).unwrap().to_owned();
  let future = use_future(
    cx,
    (query.get(), date_before.get(), date_after.get()),
    |(query, date_before, date_after)| async move {
      let query_options = QueryOptions {
        snippet_length: 400,
        date_before,
        date_after,
        ..Default::default()
      };
      let search_guard = search.read();
      let search = search_guard.read().await;
      search.query(&query, &query_options)
    },
  );

  render! {
    div {
      match future.value() {
        Some(Ok(result)) => {
          rsx! {
            "Query: {query.get()} "
            "({result.total_records} results) "
            "(elapsed: {result.elapsed:?})"
          }
        }
        _ => {
          rsx! { "Query: {query.get()}" }
        }
      }
    },
    div {
      match future.value() {
        Some(Ok(result)) => {
          rsx! { Rendered(&result.entries) }
        }
        Some(Err(err)) => {
          rsx! { "Error: {err}" }
        }
        None => {
          rsx! { "Loading..." }
        }
      }
    }
  }
}
