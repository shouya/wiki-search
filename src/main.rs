mod config;
mod index;
mod page;
mod util;
mod wiki;

use crate::{config::Config, util::Result, wiki::Wiki};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let config = Config {
    wiki_sqlite_file: "/home/shou/tmp/my_wiki.sqlite".into(),
    tantivy_index_dir: "/home/shou/tmp/index".into(),
  };

  let mut wiki = Wiki::new(&config).await?;
  let pages = wiki.list_pages().await?;

  let mut count = 0;

  dbg!(pages.len());
  for page in pages {
    if let &Some(d) = page.title_date.as_ref() {
      dbg!(page.title, d);
      count += 1;
    }
  }

  dbg!(count);

  Ok(())
}

#[allow(unused)]
fn playground() {
  use tantivy::schema::*;
  let mut schema_builder = Schema::builder();
  schema_builder.add_text_field("title", TEXT | STORED);
  schema_builder.add_text_field("text", TEXT | STORED);
  schema_builder.add_date_field("title_date", STORED);
  schema_builder.add_date_field("creation_date", STORED);
  schema_builder.add_date_field("modification_date", STORED);
  schema_builder.add_text_field("namespace", STRING | STORED);
}
