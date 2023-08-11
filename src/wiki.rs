use sqlx::{Connection, SqliteConnection};

use crate::{config::Config, page::Page, util::Result};

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

  pub async fn list_pages(&mut self) -> Result<Vec<Page>> {
    const SQL: &str = concat!(
      "SELECT",
      "    page.page_title as title, ",
      "    text.old_text as text, ",
      "    page.page_touched as page_touched, ",
      "    page.page_namespace as namespace ",
      "FROM page ",
      "LEFT JOIN slots ON page.page_latest = slots.slot_revision_id ",
      "LEFT JOIN content ON slots.slot_content_id = content.content_id ",
      "LEFT JOIN text ON ltrim(content.content_address, 'tt:') = text.old_id "
    );

    let pages = sqlx::query_as::<_, Page>(SQL)
      .fetch_all(&mut self.conn)
      .await?;

    Ok(pages)
  }
}
