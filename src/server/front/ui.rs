#![allow(non_snake_case)]

use std::marker::PhantomData;

use dioxus::{
  core::IntoDynNode,
  prelude::{
    dioxus_elements, fc_to_builder, inline_props, render, rsx, use_future,
    use_shared_state, use_shared_state_provider, use_state, Element,
    GlobalAttributes, Props, Scope, UseState,
  },
};

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

  render! {
    div {
      QueryBar {
        query: query.clone()
      }
      SearchResult {
        query: query.clone()
      }
    }
  }
}

#[inline_props]
fn QueryBar(cx: Scope, query: UseState<String>) -> Element {
  render! {
    input {
      placeholder: "Enter query here...",
      oninput: move |e| { query.set(e.value.clone()) }
    }
  }
}

struct Rendered<T>(pub T);

impl<'a, 'b> IntoDynNode<'a> for Rendered<&'b Vec<PageMatchEntry>> {
  fn into_vnode(
    self,
    cx: &'a dioxus::prelude::ScopeState,
  ) -> dioxus::core::DynamicNode<'a> {
    let mut result = String::new();
    for entry in self.0.iter() {
      let title = entry.title.highlight("<b>", "</b>");
      let text = entry.text.highlight("<b>", "</b>");

      result.push_str(&format!("[{:.2}] {}\n", entry.score, title));
      result.push_str(&format!("{}\n\n-------------\n\n", text));
    }

    rsx! {
      pre {
        dangerous_inner_html: "{result}"
      }
    }
    .into_vnode(cx)
  }
}

#[inline_props]
fn SearchResult(cx: Scope, query: UseState<String>) -> Element {
  let search = use_shared_state::<SearchRef>(cx).unwrap().to_owned();

  let future = use_future(cx, query.get(), |query| async move {
    let query_options = QueryOptions::default();
    let search_guard = search.read();
    let search = search_guard.read().await;
    search.query(&query, &query_options)
  });

  render! {
    div {
      match future.value() {
        Some(Ok(result)) => {
          rsx! { "Query: {query.get()} ({result.total_records} results) (elapsed: {result.elapsed:?})" }
        }
        _ => {
          rsx! { "Query: {query.get()}" }
        }
      }
    },
    pre {
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
