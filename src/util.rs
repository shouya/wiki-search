use std::ops::Deref;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  // sqlx errors
  #[error("sqlx error: {0}")]
  Sqlx(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub type Date = chrono::NaiveDate;
pub type DateTime = chrono::DateTime<chrono::Utc>;

// a quick extend around external types
pub struct W<T>(pub T);

impl<T> Deref for W<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl W<DateTime> {
  pub fn wiki_timestamp(&self) -> String {
    format!("{}", self.format("%Y%m%d%H%M%S"))
  }
}
