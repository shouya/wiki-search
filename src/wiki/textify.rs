#![allow(dead_code)]

use once_cell::sync::Lazy;
use regex::{Captures, Regex};

pub fn textify(source: &str) -> String {
  // Do all the following conversions in one pass.
  //
  // 1. convert newline to spaces
  // 2. remove html tags (but retain contents)
  // 3. merge consecutive spaces/newlines

  static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
      r"(?x)
      (?P<newline>\n+)|
      (?P<space>\s\s+)|
      (?P<open_tag><[^>]+>)|
      (?P<close_tag></[^>]+>)
    ",
    )
    .unwrap()
  });

  REGEX
    .replace_all(source, |caps: &Captures| {
      if let Some(newline) = caps.name("newline") {
        " ".repeat(newline.as_str().len())
      } else if let Some(space) = caps.name("space") {
        " ".repeat(space.as_str().len())
      } else if let Some(_open_tag) = caps.name("open_tag") {
        " ".to_string()
      } else if let Some(_close_tag) = caps.name("close_tag") {
        " ".to_string()
      } else {
        unreachable!()
      }
    })
    .to_string()
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_textify() {
    // assert_textify("a&quot;b", "a\"b");
    assert_textify("{{a|b=c|d=e}}", "{{a|b=c|d=e}}");
    assert_textify("{{a|b|c}}", "{{a|b|c}}");
    assert_textify("''hello''", "''hello''");
    assert_textify("'''hello'''", "'''hello'''");
    assert_textify("'''''hello'''''", "'''''hello'''''");
    assert_textify("'''''hello'''''", "'''''hello'''''");

    // bug: there is no newline between b and c
    // assert_textify("= a =\nb\n\n= c =\nd", "= a =\nb= c =\nd");
    assert_textify("= a =\nb\n\n= c =\nd", "= a = b  = c = d");
  }

  fn assert_textify(source: &str, expected: &str) {
    let actual = textify(source);
    assert_eq!(actual, expected);
  }
}
