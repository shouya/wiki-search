mod config;
mod page;
mod search;
mod util;
mod wiki;

use std::collections::HashMap;

use search::Search;

use crate::{config::Config, util::Result, wiki::Wiki};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let config = Config {
    wiki_sqlite_file: "/home/shou/tmp/my_wiki.sqlite".into(),
    index_dir: "/home/shou/tmp/index".into(),
  };

  let mut wiki = Wiki::new(&config).await?;
  let mut search = Search::new(&config)?;

  let pages = wiki.list_pages().await?;
  let page_store: HashMap<i64, _> = pages
    .clone()
    .into_iter()
    .map(|page| (page.id, page))
    .collect();

  search.index_pages(pages.into_iter())?;
  for (score, id) in search.query("truth")? {
    let page = page_store.get(&id).unwrap();
    println!("{}: {}", score, page.title);
    println!("{}", page.text);
  }

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
