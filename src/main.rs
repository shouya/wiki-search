mod config;
mod index;
mod page;
mod util;
mod wiki;

use chrono::{Days, Utc};

use crate::{config::Config, util::Result, wiki::Wiki};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let config = Config {
    wiki_sqlite_file: "/home/shou/tmp/my_wiki.sqlite".into(),
    tantivy_index_dir: "/home/shou/tmp/index".into(),
  };

  let mut wiki = Wiki::new(&config).await?;
  let date = Utc::now().checked_sub_days(Days::new(100000)).unwrap();
  let pages = wiki.list_page_titles(date).await?;

  dbg!(pages.len());

  for title in pages {
    println!("{title}");
  }

  Ok(())
}

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
