use std::path::PathBuf;

pub struct Config {
  pub wiki_sqlite_file: PathBuf,
  pub tantivy_index_dir: PathBuf,
}
