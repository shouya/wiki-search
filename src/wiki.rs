use std::path::Path;

use futures_util::StreamExt;
use sqlx::{Connection, SqliteConnection};

use crate::{page::Page, util::Result};

mod textify;

pub struct Wiki {
  conn: SqliteConnection,
  wiki_base: String,
}

impl Wiki {
  pub async fn new(
    sqlite_path: &Path,
    url_base: impl Into<String>,
  ) -> Result<Self> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
      .filename(sqlite_path)
      .read_only(true)
      .immutable(true);

    let conn = SqliteConnection::connect_with(&options).await?;
    let url_base = url_base.into();

    Ok(Self {
      conn,
      wiki_base: url_base,
    })
  }

  pub async fn list_pages(&mut self) -> Result<Vec<Page>> {
    const SQL: &str = concat!(
      "SELECT",
      "    page.page_id as id, ",
      "    replace(page.page_title, '_', ' ') as title, ",
      "    text.old_text as text, ",
      "    page.page_touched as page_touched, ",
      "    page.page_namespace as namespace ",
      "FROM page ",
      "LEFT JOIN slots ON page.page_latest = slots.slot_revision_id ",
      "LEFT JOIN content ON slots.slot_content_id = content.content_id ",
      "LEFT JOIN text ON ltrim(content.content_address, 'tt:') = text.old_id "
    );

    let mut pages = vec![];
    let mut stream = sqlx::query_as::<_, Page>(SQL).fetch(&mut self.conn);

    while let Some(Ok(mut page)) = stream.next().await {
      if page.text.trim().is_empty() {
        continue;
      }

      page.fill_url(&self.wiki_base);

      page.text = textify::textify(&page.text);
      pages.push(page);
    }

    Ok(pages)
  }
}
