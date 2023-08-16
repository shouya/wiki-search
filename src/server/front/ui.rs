use dioxus::prelude::{dioxus_elements, rsx, Element, Scope};

pub fn app(cx: Scope) -> Element {
  cx.render(rsx! {
    div {
      "hello!"
    }
  })
}
