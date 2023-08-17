use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use tracing::info;

use crate::{
  search::{QueryOptions, Search},
  util::Result,
  wiki::Wiki,
};

#[derive(Parser)]
/// command line interface
pub struct Cli {
  /// path to MediaWiki SQLite database
  #[arg(short('w'), long, env)]
  sqlite_path: PathBuf,

  /// path to search index
  #[arg(short('i'), long, env)]
  index_dir: PathBuf,

  #[command(subcommand)]
  command: Option<Command>,
}

#[derive(Subcommand)]
/// subcommands
pub enum Command {
  /// run the server (default subcommand)
  Server {
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind_addr: SocketAddr,
  },
  /// run command line query
  Query {
    /// query string
    query: String,

    #[command(flatten)]
    opts: QueryOptions,
  },
  /// re-index
  Reindex,
}

impl Cli {
  pub async fn run(self) -> Result<()> {
    match &self.command {
      None => {
        self
          .run_server(SocketAddr::from(([127, 0, 0, 1], 3000)))
          .await
      }
      Some(Command::Server { bind_addr }) => self.run_server(*bind_addr).await,
      Some(Command::Query { query, opts }) => self.run_query(query, opts).await,
      Some(Command::Reindex) => self.run_reindex().await,
    }
  }

  pub async fn wiki(&self) -> Result<Wiki> {
    Wiki::new(&self.sqlite_path).await
  }
  pub async fn search(&self) -> Result<Search> {
    Search::new(&self.index_dir)
  }

  pub async fn run_query(
    &self,
    query: &str,
    opts: &QueryOptions,
  ) -> Result<()> {
    let search = self.search().await?;

    let result = search.query(query, opts)?;
    for entry in result.entries {
      let title = entry.title.highlight("\x1b[42;30m", "\x1b[m");
      let text = entry.text.highlight("\x1b[43;30m", "\x1b[m");

      println!("[\x1b[32m{:.2}\x1b[m] {}", entry.score, title);
      println!("{}\n\n-------------\n", text);
    }

    Ok(())
  }

  pub async fn run_server(&self, bind_addr: SocketAddr) -> Result<()> {
    let wiki = self.wiki().await?;
    let search = self.search().await?;

    let server = crate::server::Server::new(bind_addr, search, wiki);
    server.run().await
  }

  pub async fn run_reindex(&self) -> Result<()> {
    use std::time::Instant;

    let mut wiki = Wiki::new(&self.sqlite_path).await?;
    let mut search = Search::new(&self.index_dir)?;

    let t = Instant::now();
    let pages = wiki.list_pages().await?;
    info!("Listed pages ({}) (spent {:?})", pages.len(), t.elapsed());

    let t = Instant::now();
    search.reindex_pages(pages)?;
    info!("Indexed pages (spent {:?})", t.elapsed());

    Ok(())
  }
}
