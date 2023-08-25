use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("sqlx error: {0}")]
  Sqlx(#[from] sqlx::Error),

  #[error("date parse error")]
  InvalidDate(String),

  #[error("tantivy error: {0}")]
  Tantivy(#[from] tantivy::TantivyError),

  #[error("invalid query: {0}")]
  InvalidQuery(#[from] tantivy::query::QueryParserError),

  #[error("hyper error: {0}")]
  Hyper(#[from] hyper::Error),

  #[error("io error: {0}")]
  Io(#[from] std::io::Error),

  #[error("generic error: `{0}`")]
  Generic(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type Date = chrono::NaiveDate;
pub type DateTime = chrono::DateTime<chrono::Utc>;
