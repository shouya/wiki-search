use crate::util::{Date, DateTime};

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Page {
  pub title: String,
  pub text: String,
  pub title_date: Option<Date>,
  pub page_touched: DateTime,
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
      -1 => Namespace::Special,
      -2 => Namespace::Media,
      _ => Namespace::Other(value),
    }
  }
}
