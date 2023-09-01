use chrono::Datelike;
use serde::Deserialize;
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

pub fn parse_date(s: &str) -> Result<tantivy::DateTime> {
  use chrono::NaiveDate;

  if s.is_empty() {
    return Err(Error::InvalidDate("empty date".into()));
  }

  let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
    .map_err(|_| Error::InvalidDate(s.to_string()))?;
  if naive_date.year() < 1700 || naive_date.year() > 2200 {
    return Err(Error::InvalidDate(s.to_string()));
  }

  let datetime = naive_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
  let datetime = tantivy::DateTime::from_timestamp_secs(datetime.timestamp());

  Ok(datetime)
}

pub fn deserialize_date<'de, D>(
  deserializer: D,
) -> Result<Option<tantivy::DateTime>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  if s.is_empty() {
    return Ok(None);
  }

  let date_time = parse_date(&s).map_err(serde::de::Error::custom)?;

  Ok(Some(date_time))
}
