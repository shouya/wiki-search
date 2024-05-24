use std::{path::Path, time::Duration};

use futures_util::StreamExt;
use sqlx::{pool::PoolOptions, SqlitePool};

use crate::{page::Page, util::Result};

mod textify;

pub struct Wiki {
  pool: SqlitePool,
  wiki_base: String,
}

impl Wiki {
  pub async fn new(
    sqlite_path: &Path,
    wiki_base: impl Into<String>,
  ) -> Result<Self> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
      .filename(sqlite_path)
      .read_only(true);

    let pool_options =
      PoolOptions::new().idle_timeout(Some(Duration::from_secs(5 * 60)));
    let pool = pool_options.connect_lazy_with(options);

    let wiki_base = wiki_base.into();

    Ok(Self { pool, wiki_base })
  }

  pub async fn list_pages(&mut self) -> Result<Vec<Page>> {
    const SQL: &str = concat!(
      "SELECT",
      "    page.page_id as id, ",
      "    replace(page.page_title, '_', ' ') as title, ",
      "    text.old_text as text, ",
      "    page.page_touched as updated, ",
      "    page.page_namespace as namespace, ",
      "    (SELECT GROUP_CONCAT(categorylinks.cl_to, '<|||>')
            FROM categorylinks
            WHERE categorylinks.cl_from = page.page_id) as categories ",
      "FROM page ",
      "LEFT JOIN slots ON page.page_latest = slots.slot_revision_id ",
      "LEFT JOIN content ON slots.slot_content_id = content.content_id ",
      "LEFT JOIN text ON ltrim(content.content_address, 'tt:') = text.old_id",
    );

    let mut pages = vec![];
    let mut stream = sqlx::query_as::<_, Page>(SQL).fetch(&self.pool);

    while let Some(val) = stream.next().await {
      let mut page = val?;
      if page.text.trim().is_empty() {
        continue;
      }

      page.fill_url(&self.wiki_base);

      page.text = textify::textify(&page.text);
      pages.push(page);
    }

    Ok(pages)
  }

  // return a number that increases when any update is made that
  // requires reindexing.
  pub async fn latest_revision(&self) -> Result<u32> {
    // A O(1) way to get the primary key sequence value, c.f.
    // https://stackoverflow.com/a/2217015
    const SQL: &str = "SELECT seq FROM sqlite_sequence WHERE name = 'revision'";
    let val: (u32,) = sqlx::query_as(SQL).fetch_one(&self.pool).await?;
    Ok(val.0)
  }
}
