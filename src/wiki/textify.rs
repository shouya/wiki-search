use parse_wiki_text::{
  Configuration, DefinitionListItem, ListItem, Node, Parameter, TableCaption,
  TableCell, TableRow,
};

trait Textify {
  fn textify(self, source: &str, buffer: &mut String);
}

impl Textify for DefinitionListItem<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    self.nodes.textify(source, buffer);
  }
}

impl Textify for ListItem<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    self.nodes.textify(source, buffer);
  }
}

impl Textify for Parameter<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    if let Some(name) = self.name {
      name.textify(source, buffer);
      buffer.push('=');
    };

    self.value.textify(source, buffer);
  }
}

impl<T> Textify for Vec<T>
where
  T: Textify,
{
  fn textify(self, source: &str, buffer: &mut String) {
    for val in self {
      val.textify(source, buffer);
    }
  }
}

impl Textify for TableCell<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    self.content.textify(source, buffer);
  }
}
impl Textify for TableRow<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    SepBy(" | ", self.cells).textify(source, buffer);
  }
}
impl Textify for TableCaption<'_> {
  fn textify(self, source: &str, buffer: &mut String) {
    self.content.textify(source, buffer);
  }
}

struct SepBy<T>(&'static str, Vec<T>);

impl<T> Textify for SepBy<T>
where
  T: Textify,
{
  fn textify(self, source: &str, buffer: &mut String) {
    for (i, item) in self.1.into_iter().enumerate() {
      if i != 0 {
        buffer.push_str(self.0);
      }
      item.textify(source, buffer);
    }
  }
}

struct LineSep<T>(Vec<T>);

impl<T> Textify for LineSep<T>
where
  T: Textify,
{
  fn textify(self, source: &str, buffer: &mut String) {
    for item in self.0 {
      item.textify(source, buffer);
      buffer.push('\n');
    }
  }
}

impl<'a> Textify for Node<'a> {
  fn textify(self, source: &str, buffer: &mut String) {
    use Node::*;
    match self {
      Bold { end, start } => buffer.push_str(&source[start..end]),
      BoldItalic { end, start } => buffer.push_str(&source[start..end]),
      Category { .. } => {}
      CharacterEntity { character, .. } => buffer.push(character),
      Comment { end, start } => buffer.push_str(&source[start..end]),
      DefinitionList { items, .. } => items.textify(source, buffer),
      EndTag { .. } => {}
      ExternalLink { nodes, .. } => nodes.textify(source, buffer),
      Heading {
        end, nodes, start, ..
      } => {
        buffer.push_str(&source[start..end]);
        buffer.push('\n');
        nodes.textify(source, buffer);
      }
      HorizontalDivider { .. } => {
        buffer.push_str("----------");
      }
      Image { target, text, .. } => {
        buffer.push_str(&format!("IMAGE: {}", target));
        text.textify(source, buffer);
      }
      Italic { end, start } => buffer.push_str(&source[start..end]),
      Link { target, text, .. } => {
        text.textify(source, buffer);
        buffer.push_str(&format!("({target})"));
      }
      MagicWord { end, start } => buffer.push_str(&source[start..end]),
      OrderedList { items, .. } => items.textify(source, buffer),
      ParagraphBreak { .. } => buffer.push_str("\n\n"),
      Parameter { .. } => {}
      Preformatted { .. } => {}
      Redirect { target, .. } => {
        buffer.push_str(&format!("REDIRECT: {}", target))
      }
      StartTag { .. } => {}
      Table { captions, rows, .. } => {
        LineSep(captions).textify(source, buffer);
        LineSep(rows).textify(source, buffer);
      }
      Tag { .. } => {}
      Template {
        name, parameters, ..
      } => {
        buffer.push_str("{{");
        name.textify(source, buffer);
        if parameters.is_empty() {
          buffer.push_str("}}");
        } else {
          buffer.push('|');
          SepBy("|", parameters).textify(source, buffer);
          buffer.push_str("}}");
        }
      }
      Text { value, .. } => buffer.push_str(value),
      UnorderedList { items, .. } => LineSep(items).textify(source, buffer),
    }
  }
}

pub fn textify(source: &str) -> String {
  let parser_config = Configuration::default();
  let parser_output = parser_config.parse(source);

  let mut buffer = String::new();
  parser_output.nodes.textify(source, &mut buffer);
  buffer
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_textify() {
    assert_textify("a&quot;b", "a\"b");
    assert_textify("{{a|b=c|d=e}}", "{{a|b=c|d=e}}");
    assert_textify("{{a|b|c}}", "{{a|b|c}}");
    assert_textify("''hello''", "''hello''");
    assert_textify("'''hello'''", "'''hello'''");
    assert_textify("'''''hello'''''", "'''''hello'''''");
    assert_textify("'''''hello'''''", "'''''hello'''''");
  }

  fn assert_textify(source: &str, expected: &str) {
    let actual = textify(source);
    assert_eq!(actual, expected);
  }
}
