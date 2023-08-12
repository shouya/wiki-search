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

  #[error("generic error: `{0}`")]
  Generic(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub type Date = chrono::NaiveDate;
pub type DateTime = chrono::DateTime<chrono::Utc>;
