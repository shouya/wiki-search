mod cli;
mod config;
mod page;
mod search;
mod util;
mod wiki;

use cli::Cli;

use crate::{config::Config, util::Result};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let config = Config {
    wiki_sqlite_file: "/home/shou/tmp/my_wiki.sqlite".into(),
    index_dir: "/home/shou/tmp/index".into(),
  };

  let cli: Cli = argh::from_env();
  cli.run(&config).await?;

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
