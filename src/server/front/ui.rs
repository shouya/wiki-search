#![allow(non_snake_case)]

use std::{marker::PhantomData, time::Duration};

use dioxus::{
  core::IntoDynNode,
  prelude::{
    dioxus_elements, fc_to_builder, inline_props, render, rsx, use_eval,
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
  // to convince dioxus that this prop contains a ref, so do not
  // attempt to diff it.
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
        date_after: date_after.clone(),
        offset: 0
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

  let eval = use_eval(cx).clone();

  cx.spawn({
    let eval = eval.clone();
    async move {
      eval(
        "document.querySelectorAll('.query-date input:not(.datepicker-input)').forEach(e =>
           new Datepicker(e, {format: 'yyyy-mm-dd'})
         );",
      )
      .unwrap()
      .await
      .unwrap();
    }
  });

  render! {
    div {
      class: "query-bar",
      div {
        class: "user-input",
        div {
          class: "query-input",
          label {
            span { class: "query-input-label", "Query: " }
            input {
              placeholder: "Enter query here...",
              value: "{query}",
              oninput: move |e| { query.set(e.value.clone()) }
            }
          }
          button {
            onclick: move |_| {
              query.set(String::new());
            },
            "Clear"
          }
        }
        div {
          class: "query-date",
          label {
            span { class: "query-date-label", "Before: " }
            input {
              placeholder: "2023-01-02",
              onchange: move |e| { set_date(date_before.clone(), &e.value) },
              oninput: move |e| { set_date(date_after.clone(), &e.value) },
            }
          }
          label {
            span { class: "query-date-label", "After: " }
            input {
              placeholder: "2023-01-02",
              onchange: move |e| { set_date(date_after.clone(), &e.value) },
              oninput: move |e| { set_date(date_after.clone(), &e.value) },
            }
          }
        }
      }
      ReindexButton {}
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

type EvalCreator = std::rc::Rc<
  dyn Fn(&str) -> Result<dioxus::prelude::UseEval, dioxus::prelude::EvalError>,
>;

async fn is_visible(eval: EvalCreator, element_id: &str) -> bool {
  let signal = eval(&format!(
    r#"
!function() {{
  const signal = new AbortController();
  window.addEventListener('scroll', function() {{
    const el = document.getElementById("{element_id}");
    if (el == null) {{
      signal.abort();
      return;
    }}
    const rect = el.getBoundingClientRect();
    const {{top, bottom}} = rect;
    if ((top >= 0) && (bottom <= window.innerHeight)) {{
      signal.abort();
      dioxus.send(true);
    }}
  }}, {{ signal: signal.signal }})
}}();
"#,
  ))
  .unwrap();
  let jsvalue = signal.recv().await.unwrap();
  jsvalue.as_bool().unwrap()
}

#[inline_props]
fn SearchResult(
  cx: Scope,
  query: UseState<String>,
  date_before: UseState<Option<DateTime>>,
  date_after: UseState<Option<DateTime>>,
  offset: usize,
) -> Element {
  let visible = use_state(cx, || *offset == 0);
  let search = use_shared_state::<SearchRef>(cx).unwrap().to_owned();
  let eval = use_eval(cx).clone();
  let element_id = format!("search-result-{}", offset);

  let future = use_future(
    cx,
    (
      query.get(),
      date_before.get(),
      date_after.get(),
      offset,
      visible.get(),
    ),
    |(query, date_before, date_after, offset, visible)| async move {
      if !visible {
        return None;
      }

      let query_options = QueryOptions {
        snippet_length: 400,
        date_before,
        date_after,
        offset,
        ..Default::default()
      };
      let search_guard = search.read();
      let search = search_guard.read().await;
      Some(search.query(&query, &query_options))
    },
  );

  let visible_cloned = visible.clone();
  let element_id_cloned = element_id.clone();
  cx.push_future({
    async move {
      is_visible(eval, &element_id_cloned).await;
      visible_cloned.set(true);
    }
  });

  render! {
    hr {}
    match future.value() {
      // Future not ready, loading.
      None => { rsx! { "Loading..." } }

      // Element currently not visible, lazy load suspended.
      Some(None) => {
        rsx! {
          div {
            id: "{element_id}",
            onclick: move |_| { visible.set(true) },
            "Click to load next page"
          }
        }
      }

      // Element visible and result is ready, render results.
      Some(Some(Ok(result))) => {
        rsx! {
          div  {
            "Query: {query.get()} "
            "({result.remaining} results left) "
            "(elapsed: {result.elapsed:?})"
          }

          Rendered(&result.entries)

          // if more pages are available, render a button to load the next page
          match result.new_offset {
            None => rsx! { "Last page reached!" },
            Some(new_offset) => {
              rsx! {
                SearchResult {
                  query: query.clone(),
                  date_before: date_before.clone(),
                  date_after: date_after.clone(),
                  offset: new_offset
                }
              }
            }
          }
        }
      }

      // Loading failed.
      Some(Some(Err(e))) => {
        rsx! {
          div {
            "Error: {e}"
          }
        }
      }
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReloadStatus {
  Initial,
  Loading,
  Done(Duration),
}

#[inline_props]
fn ReindexButton(cx: Scope) -> Element {
  let wiki = use_shared_state::<WikiRef>(cx).unwrap().to_owned();
  let search = use_shared_state::<SearchRef>(cx).unwrap().to_owned();
  let reload_status = use_state(cx, || ReloadStatus::Initial);

  let page_count = use_future(cx, reload_status.get(), |_reload_status| {
    let search = search.clone();
    async move {
      let guard1 = search.read();
      let guard2 = guard1.read().await;
      guard2.page_count().unwrap_or_default()
    }
  });

  let reindex_button = |dur: Option<Duration>| {
    rsx! {
      button {
        onclick: move |_| {
          reload_status.set(ReloadStatus::Loading);
          cx.spawn({
            let wiki = wiki.clone();
            let search = search.clone();
            let reload_status = reload_status.clone();
            async move {
              let start = std::time::Instant::now();
              let wiki_guard = wiki.read();
              let pages = wiki_guard.lock().await.list_pages().await.unwrap();
              let search_guard = search.read();
              search_guard.write().await.reindex_pages(pages).unwrap();
              reload_status.set(ReloadStatus::Done(start.elapsed()));
            }
          });
        },

        match dur {
          None => rsx! { "Reindex" },
          Some(dur) => {
            let dur = format!("{:.2?}", dur);
            rsx! { "Reindex (took: {dur})" }
          }
        }
      }
    }
  };

  render! {
    div {
      class: "reindex-button",
      match reload_status.get() {
        ReloadStatus::Loading => rsx! { "Reindexing..." },
        ReloadStatus::Initial => reindex_button(None),
        ReloadStatus::Done(dur) => reindex_button(Some(*dur))
      }
      div {
        class: "page-count",
        "Indexed pages: "
        match page_count.value() {
          None => rsx! { "loading..." },
          Some(n) => rsx! { "{n}" }
        }
      }
    }
  }
}
