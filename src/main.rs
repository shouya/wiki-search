mod cli;
mod page;
mod search;
mod server;
mod util;
mod wiki;

use clap::Parser;
use cli::Cli;

use crate::util::Result;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  tracing_subscriber::fmt().without_time().init();

  #[cfg(feature = "dotenv")]
  let _ = dotenv::dotenv();

  let cli = Cli::parse();
  cli.run().await?;

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
