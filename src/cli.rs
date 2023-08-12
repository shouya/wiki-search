use argh::FromArgs;

use crate::{config::Config, search::Search, util::Result, wiki::Wiki};

#[derive(FromArgs)]
/// command line interface
pub struct Cli {
  #[argh(subcommand)]
  command: Subcommand,
}

#[derive(FromArgs)]
/// subcommands
#[argh(subcommand)]
pub enum Subcommand {
  /// run the server
  Server(Server),
  /// run command line query
  Query(Query),
  /// re-index
  Reindex(Reindex),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "server")]
/// run the server
pub struct Server {}

#[derive(FromArgs)]
#[argh(subcommand, name = "query")]
/// run command line query
pub struct Query {
  #[argh(positional)]
  /// query string
  pub query: String,

  /// disable highlighting on matched terms
  #[argh(option, short = 'c', default = "false")]
  pub no_color: bool,

  /// number of results to return
  #[argh(option, short = 'n', default = "10")]
  pub count: usize,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "reindex")]
/// re-index
pub struct Reindex {}

impl Cli {
  pub async fn run(self, config: &Config) -> Result<()> {
    match self.command {
      Subcommand::Server(_) => self.run_server(config).await,
      Subcommand::Query(ref query) => self.run_query(query, config),
      Subcommand::Reindex(_) => self.run_reindex(config).await,
    }
  }

  pub fn run_query(&self, query: &Query, config: &Config) -> Result<()> {
    let search = Search::new(config)?;

    for mut entry in search.query(&query.query, query.count)? {
      entry
        .body_snippet
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
      println!("{}\n\n-------------\n", entry.body_snippet.to_html());
    }

    Ok(())
  }

  pub async fn run_server(&self, _config: &Config) -> Result<()> {
    todo!()
  }

  pub async fn run_reindex(&self, config: &Config) -> Result<()> {
    let mut wiki = Wiki::new(config).await?;
    let mut search = Search::new(config)?;
    let pages = wiki.list_pages().await?;
    search.index_pages(pages.into_iter())?;

    Ok(())
  }
}
