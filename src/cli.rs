use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use tracing::info;

use crate::{search::Search, util::Result, wiki::Wiki};

#[derive(Parser)]
/// command line interface
pub struct Cli {
  /// path to mediawiki sqlite database
  #[arg(short, long, env)]
  sqlite_path: PathBuf,

  /// path to search index
  #[arg(short, long, env)]
  index_dir: PathBuf,

  #[command(subcommand)]
  command: Command,
}

#[derive(Args)]
pub struct QueryOpts {
  /// disable highlighting on matched terms
  #[arg(short('c'), long, default_value_t = false)]
  no_color: bool,

  /// number of results to return
  #[arg(short('n'), long, default_value_t = 10)]
  count: usize,
}

#[derive(Subcommand)]
/// subcommands
pub enum Command {
  /// run the server
  Server {
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind_addr: SocketAddr,
  },
  /// run command line query
  Query {
    /// query string
    query: String,

    #[command(flatten)]
    opts: QueryOpts,
  },
  /// re-index
  Reindex,
}

impl Cli {
  pub async fn run(self) -> Result<()> {
    match &self.command {
      Command::Server { bind_addr } => self.run_server(*bind_addr).await,
      Command::Query { query, opts } => self.run_query(query, opts).await,
      Command::Reindex => self.run_reindex().await,
    }
  }

  pub async fn wiki(&self) -> Result<Wiki> {
    Wiki::new(&self.sqlite_path).await
  }
  pub async fn search(&self) -> Result<Search> {
    Search::new(&self.index_dir)
  }

  pub async fn run_query(&self, query: &str, opts: &QueryOpts) -> Result<()> {
    let search = self.search().await?;

    for mut entry in search.query(query, opts.count)? {
      entry
        .text_snippet
        .set_snippet_prefix_postfix("\x1b[42;30m", "\x1b[m");
      entry
        .title_snippet
        .set_snippet_prefix_postfix("\x1b[42;30m", "\x1b[m");

      let title = if entry.title_snippet.is_empty() {
        entry.title
      } else {
        entry.title_snippet.to_html()
      };

      println!("[\x1b[32m{:.2}\x1b[m] {}", entry.score, title);
      println!("{}\n\n-------------\n", entry.text_snippet.to_html());
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
    use std::{fs, time::Instant};

    fs::remove_dir_all(&self.index_dir).ok();
    fs::create_dir_all(&self.index_dir).ok();

    let mut wiki = Wiki::new(&self.sqlite_path).await?;
    let mut search = Search::new(&self.index_dir)?;

    let t = Instant::now();
    let pages = wiki.list_pages().await?;
    info!("Listed pages ({}) (spent {:?})", pages.len(), t.elapsed());

    let t = Instant::now();
    search.index_pages(pages.into_iter())?;
    info!("Indexed pages (spent {:?})", t.elapsed());

    Ok(())
  }
}
