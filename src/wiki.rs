use std::path::Path;

use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use sqlx::{Connection, SqliteConnection};

use crate::{page::Page, util::Result};

mod textify;

pub struct Wiki {
  conn: SqliteConnection,
}

impl Wiki {
  pub async fn new(sqlite_path: &Path) -> Result<Self> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
      .filename(sqlite_path)
      .read_only(true)
      .immutable(true);

    let conn = SqliteConnection::connect_with(&options).await?;

    Ok(Self { conn })
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

    let mut pages = sqlx::query_as::<_, Page>(SQL)
      .fetch_all(&mut self.conn)
      .await?;

    pages.par_iter_mut().for_each(|page| {
      // convert mediawiki to plain text
      page.text = textify::textify(&page.text);
    });

    Ok(pages)
  }
}
