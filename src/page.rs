use chrono::NaiveDateTime;

use crate::util::{Date, DateTime, Error};

#[derive(Clone, Copy, Debug, derive_more::Display)]
pub enum Namespace {
  Main,
  User,
  File,
  Template,
  Category,
  Special,
  MediaWiki,
  Help,
  Module,
  MainTalk,
  UserTalk,
  FileTalk,
  TemplateTalk,
  CategoryTalk,
  MediaWikiTalk,
  HelpTalk,
  ModuleTalk,
  Media,
  Other(i32),
}

#[derive(
  Clone,
  Debug,
  PartialEq,
  derive_more::From,
  derive_more::Into,
  derive_more::AsRef,
  derive_more::AsMut,
)]
pub struct TitleDate(pub Option<Date>);

#[derive(
  Clone,
  Debug,
  derive_more::From,
  derive_more::Into,
  derive_more::AsRef,
  derive_more::AsMut,
)]
pub struct WikiTimestamp(pub DateTime);

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Page {
  pub id: i64,
  pub title: String,
  pub text: String,
  #[sqlx(rename = "title", try_from = "String")]
  pub title_date: TitleDate,
  #[sqlx(try_from = "String")]
  pub page_touched: WikiTimestamp,
  #[sqlx(try_from = "i32")]
  pub namespace: Namespace,
}

impl From<i32> for Namespace {
  fn from(value: i32) -> Self {
    match value {
      0 => Namespace::Main,
      2 => Namespace::User,
      4 => Namespace::Template,
      6 => Namespace::File,
      8 => Namespace::MediaWiki,
      10 => Namespace::Template,
      12 => Namespace::Help,
      14 => Namespace::Category,
      1 => Namespace::MainTalk,
      3 => Namespace::UserTalk,
      5 => Namespace::TemplateTalk,
      7 => Namespace::FileTalk,
      9 => Namespace::MediaWikiTalk,
      11 => Namespace::TemplateTalk,
      13 => Namespace::HelpTalk,
      15 => Namespace::CategoryTalk,
      828 => Namespace::Module,
      829 => Namespace::ModuleTalk,
      -1 => Namespace::Special,
      -2 => Namespace::Media,
      i => Namespace::Other(i),
    }
  }
}

impl Namespace {
  pub fn to_prefix(self) -> &'static str {
    use Namespace::*;

    match self {
      Main => "",
      User => "User:",
      File => "File:",
      Template => "Template:",
      Category => "Category:",
      Special => "Special:",
      MediaWiki => "MediaWiki:",
      Help => "Help:",
      Module => "Module:",
      MainTalk => "Talk:",
      UserTalk => "User_talk:",
      FileTalk => "File_talk:",
      TemplateTalk => "Template_talk:",
      CategoryTalk => "Category_talk:",
      MediaWikiTalk => "MediaWiki_talk:",
      HelpTalk => "Help_talk:",
      ModuleTalk => "Module_talk:",
      Media => "Media:",
      Other(_) => "Unknown",
    }
  }
}

const TITLE_DATE_FORMATS: [&str; 7] = [
  // Jan 2, 2023
  "%b %-d, %Y",
  // Jan 02, 2023
  "%b %0d, %Y",
  // 2023-01-02 (unused?)
  "%Y-%m-%d",
  // 2023-01-02 (unused?)
  "%Y年%m月%d日",
  // 2023-01 (e.g. category)
  "%Y-%m",
  // 2023 Jan
  "%Y %b",
  // 2023 (e.g. category)
  "%Y",
];

impl TryFrom<String> for TitleDate {
  type Error = Error;

  // try parse the prefix of the value with above formats
  fn try_from(value: String) -> Result<Self, Self::Error> {
    let value = value.replace('_', " ");

    use chrono::format::{parse_and_remainder, Parsed, StrftimeItems};

    for format in TITLE_DATE_FORMATS.iter() {
      let mut parsed = Parsed::new();
      let fmt = StrftimeItems::new(format);

      let Ok(_) = parse_and_remainder(&mut parsed, &value, fmt) else {
        continue;
      };

      parsed.month.get_or_insert(1);
      parsed.day.get_or_insert(1);

      let Some(year) = parsed.year else { continue };

      // range of nanosecond-precision unix timestamps in i64
      if !(1678..=2262).contains(&year) {
        // this is almost definitely a mismatch
        continue;
      }

      let Ok(date) = parsed.to_naive_date() else {
        continue;
      };

      return Ok(TitleDate(Some(date)));
    }

    Ok(TitleDate(None))
  }
}

impl TryFrom<String> for WikiTimestamp {
  type Error = Error;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    const FORMAT: &str = "%Y%m%d%H%M%S";
    let date_time = NaiveDateTime::parse_from_str(&value, FORMAT)
      .map_err(|_e| Error::InvalidDate(value))?;
    Ok(WikiTimestamp(date_time.and_utc()))
  }
}

impl TitleDate {
  pub fn timestamp(&self) -> Option<i64> {
    self
      .0
      .map(|date| date.and_hms_opt(0, 0, 0).unwrap().timestamp())
  }
}

impl WikiTimestamp {
  pub fn timestamp(&self) -> i64 {
    self.0.timestamp()
  }
}

impl Page {
  pub fn to_url(&self, base: &str) -> String {
    format!("{}{}{}", base, self.namespace.to_prefix(), self.title)
  }
}

#[cfg(test)]
mod test {
  #[test]
  fn title_date_parsing() {
    use super::TitleDate;
    use chrono::NaiveDate;

    let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let title_date = &TitleDate(Some(date));

    let assert_parse = |title_date, s: &str| {
      let parsed_title_date: TitleDate = s.to_string().try_into().unwrap();
      assert_eq!(title_date, &parsed_title_date);
    };

    assert_parse(title_date, "Jan 1, 2023");
    assert_parse(title_date, "Jan 01, 2023");
    assert_parse(title_date, "Jan 1, 2023/Note");
    assert_parse(title_date, "2023-01");
    assert_parse(title_date, "2023");
  }
}
