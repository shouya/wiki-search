use std::{sync::Arc, time::Duration};

use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::{search::Search, util::Result, wiki::Wiki};

pub struct Reindexer {
  search: Arc<RwLock<Search>>,
  wiki: Arc<Mutex<Wiki>>,
  reindex_interval: Duration,
}

// reindex every hour
const DEFAULT_REINDEX_INTERVAL: Duration = Duration::from_secs(60 * 60);

impl Reindexer {
  pub fn new(search: Arc<RwLock<Search>>, wiki: Arc<Mutex<Wiki>>) -> Reindexer {
    Self {
      search,
      wiki,
      reindex_interval: DEFAULT_REINDEX_INTERVAL,
    }
  }

  #[allow(unused)]
  pub fn with_interval(mut self, interval: Duration) -> Self {
    self.reindex_interval = interval;
    self
  }

  async fn run(self) {
    info!("Reindexer started at interval {:?}", self.reindex_interval);

    loop {
      match self.reindex().await {
        Ok(_) => {}
        Err(e) => {
          warn!("scheduled reindex failed: {}", e);
        }
      }
      tokio::time::sleep(self.reindex_interval).await;
    }
  }

  async fn reindex(&self) -> Result<()> {
    let mut wiki = self.wiki.lock().await;
    let mut search = self.search.write().await;
    let revision = wiki.latest_revision().await?;
    if !search.requires_reindex(revision) {
      debug!("no reindex required");
      return Ok(());
    }

    let pages = wiki.list_pages().await?;
    search.reindex_pages(pages, revision)?;

    let page_count = search.page_count()?;
    info!("reindex successful, indexed {} pages", page_count);
    Ok(())
  }

  pub fn start(self) {
    tokio::task::spawn(self.run());
  }
}
