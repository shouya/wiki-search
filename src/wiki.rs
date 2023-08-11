use chrono::DateTime;
use sqlx::{Connection, SqliteConnection};

use crate::{
  config::Config,
  page::Page,
  util::{Result, W},
};

pub struct Wiki {
  conn: SqliteConnection,
}

impl Wiki {
  pub async fn new(config: &Config) -> Result<Self> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
      .filename(&config.wiki_sqlite_file)
      .read_only(true)
      .immutable(true);

    let conn = SqliteConnection::connect_with(&options).await?;

    Ok(Self { conn })
  }

  pub async fn list_page_titles(
    &mut self,
    after: DateTime<chrono::Utc>,
  ) -> Result<Vec<String>> {
    let names = sqlx::query_as::<_, (String,)>(
      "SELECT page_title FROM page WHERE page_touched > ?",
    )
    .bind(W(after).wiki_timestamp())
    .fetch_all(&mut self.conn)
    .await?
    .into_iter()
    .map(|(name,)| name)
    .collect();

    Ok(names)
  }

  pub async fn list_pages(&self) -> Result<Vec<Page>> {
    unimplemented!()
  }
}
