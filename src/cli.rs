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

  /// base prefix to wiki site
  #[arg(short('b'), long, env)]
  wiki_base: String,

  #[command(subcommand)]
  command: Option<Command>,
}

#[derive(Subcommand)]
/// subcommands
pub enum Command {
  /// run the server (default subcommand)
  Server {
    #[arg(short, long, default_value = "127.0.0.1:3000", env)]
    bind_addr: SocketAddr,

    #[arg(short, long, default_value = "true", env)]
    auto_reindex: bool,
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
  pub async fn run(mut self) -> Result<()> {
    match &self.command {
      None => {
        let app = std::env::args().next().unwrap();
        self.update_from([&app, "server"]);
        self.run_command().await
      }
      Some(_) => self.run_command().await,
    }
  }

  pub async fn run_command(self) -> Result<()> {
    match &self.command {
      None => unreachable!("no subcommand"),
      Some(Command::Server {
        bind_addr,
        auto_reindex,
      }) => self.run_server(*bind_addr, *auto_reindex).await,
      Some(Command::Query { query, opts }) => self.run_query(query, opts).await,
      Some(Command::Reindex) => self.run_reindex().await,
    }
  }

  pub async fn wiki(&self) -> Result<Wiki> {
    Wiki::new(&self.sqlite_path, &self.wiki_base).await
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

      println!("[\x1b[32m{}\x1b[m]", title);
      println!("{}\n\n-------------\n", text);
    }

    Ok(())
  }

  pub async fn run_server(
    &self,
    bind_addr: SocketAddr,
    auto_reindex: bool,
  ) -> Result<()> {
    let wiki = self.wiki().await?;
    let search = self.search().await?;

    let server = crate::server::Server::new(bind_addr, search, wiki);
    if auto_reindex {
      let reindexer = server.spin_off_reindexer();
      reindexer.start();
    }

    server.run().await
  }

  pub async fn run_reindex(&self) -> Result<()> {
    use std::time::Instant;

    if !self.index_dir.exists() {
      let _ = std::fs::create_dir_all(&self.index_dir);
    }

    let mut wiki = self.wiki().await?;
    let mut search = self.search().await?;

    let t = Instant::now();
    let revision = wiki.latest_revision().await?;
    let pages = wiki.list_pages().await?;
    info!("Listed pages ({}) (spent {:?})", pages.len(), t.elapsed());

    let t = Instant::now();
    search.reindex_pages(pages, revision)?;
    info!("Indexed pages (spent {:?})", t.elapsed());

    Ok(())
  }
}
