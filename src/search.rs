use std::{
  ops::{Bound, Range},
  path::Path,
};

use clap::Args;
use serde::Deserialize;
use tantivy::{
  collector::MultiCollector,
  directory::MmapDirectory,
  query::{AllQuery, Query},
  schema::{Field, Schema},
  tokenizer::TextAnalyzer,
  DateTime, DocAddress, Document, Index, IndexWriter, Order, Searcher, Snippet,
  SnippetGenerator,
};
use tantivy_jieba::JiebaTokenizer;

use crate::{page::Page, util::Result};

pub struct Fields {
  id: Field,
  title: Field,
  text: Field,
  title_date: Field,
  updated: Field,
  namespace: Field,
  url: Field,
  category: Field,
}

pub struct Search {
  #[allow(unused)]
  schema: Schema,
  fields: Fields,
  index: Index,
  revision: u32,
}

#[derive(Debug)]
pub struct PageMatchResult {
  pub entries: Vec<PageMatchEntry>,
  pub new_offset: Option<usize>,
  pub remaining: usize,
  pub elapsed: std::time::Duration,
}

#[derive(Debug)]
pub struct PageMatchEntry {
  pub namespace: String,
  pub title: MatchSnippet,
  pub text: MatchSnippet,
  pub url: String,
  pub page_id: i64,
}

#[derive(derive_more::Constructor, Debug)]
pub struct MatchSnippet {
  source: String,
  snippet: Snippet,
  max_length: usize,
}

impl MatchSnippet {
  pub fn highlight(&self, prefix: &str, suffix: &str) -> String {
    let highlights = collapse_overlapped_ranges(self.snippet.highlighted());
    let fragment = self.snippet.fragment();

    if highlights.is_empty() {
      return self.source.chars().take(self.max_length).collect();
    }

    let mut out = String::with_capacity(fragment.len() + 20);
    let mut start_from = 0;

    for item in highlights {
      out.push_str(&fragment[start_from..item.start]);
      out.push_str(prefix);
      out.push_str(&fragment[item.clone()]);
      out.push_str(suffix);
      start_from = item.end;
    }
    out.push_str(&fragment[start_from..fragment.len()]);
    out
  }
}

// This same struct was used in three places:
//
// 1. Cli, requires derive(Args)
// 2. Web handler (search query), requires derive(Deserialize)
// 3. Here as a public search API
//
// I know putting the Cli options and deserialize here is a bit leaky,
// but given that the Cli query options and the Search query options
// are actually the same concept, it makes sense to put them together.
//
// It would be the best if I can specify the clap options in the Cli
// module and the deserialize implementations in handler. They're
// doable by manually implementing the traits, but it's a lot more
// convenient to use the derive macros.

#[derive(Debug, Clone, Args, Deserialize)]
pub struct QueryOptions {
  /// offset of results
  #[clap(short('s'), long, default_value_t = 0)]
  #[serde(default)]
  pub offset: usize,

  /// number of results
  #[clap(short('n'), long, default_value_t = 10)]
  pub count: usize,

  /// max length of snippet
  #[clap(short('l'), long, default_value_t = 400)]
  pub snippet_length: usize,

  /// search pages with title date before this date
  #[serde(deserialize_with = "crate::util::deserialize_date")]
  #[clap(long, value_parser = crate::util::parse_date)]
  pub date_before: Option<tantivy::DateTime>,

  /// search pages with title date after this date
  #[serde(deserialize_with = "crate::util::deserialize_date")]
  #[clap(long, value_parser = crate::util::parse_date)]
  pub date_after: Option<tantivy::DateTime>,

  /// fuzzy search
  #[clap(short('f'), long, default_value_t)]
  #[serde(default)]
  pub fuzzy: bool,
}

impl Default for QueryOptions {
  fn default() -> Self {
    QueryOptions {
      offset: 0,
      count: 10,
      snippet_length: 100,
      date_before: None,
      date_after: None,
      fuzzy: false,
    }
  }
}

impl QueryOptions {
  pub fn date_specified(&self) -> bool {
    self.date_before.is_some() || self.date_after.is_some()
  }
}

impl Search {
  pub fn new(index_dir: &Path) -> Result<Self> {
    if !index_dir.exists() {
      std::fs::create_dir_all(index_dir)?;
    }

    let (fields, schema) = build_schema();
    let dir = MmapDirectory::open(index_dir)
      .map_err(|e| e.to_string())
      .unwrap();

    let index = Index::open_or_create(dir, schema.clone())?;
    index.tokenizers().register("text", text_tokenizer());
    index.tokenizers().register("casei", casei_tokenizer());
    index
      .fast_field_tokenizer()
      .register("casei", casei_tokenizer());

    Ok(Search {
      fields,
      schema,
      index,
      revision: 0,
    })
  }

  pub fn requires_reindex(&self, latest_revision: u32) -> bool {
    self.revision < latest_revision
  }

  pub fn reindex_pages(
    &mut self,
    pages: impl IntoIterator<Item = Page>,
    revision: u32,
  ) -> Result<()> {
    let mut writer = self.index.writer(128_000_000)?;
    writer.delete_all_documents()?;
    self.index_pages_with(&writer, pages)?;
    writer.commit()?;
    self.revision = revision;
    Ok(())
  }

  fn index_page_with(&self, writer: &IndexWriter, page: Page) -> Result<()> {
    writer.add_document(self.make_doc(page)?)?;

    Ok(())
  }

  fn index_pages_with(
    &mut self,
    writer: &IndexWriter,
    pages: impl IntoIterator<Item = Page>,
  ) -> Result<()> {
    for page in pages {
      self.index_page_with(writer, page)?;
    }

    Ok(())
  }

  fn parse_query(
    &self,
    query: &str,
    options: &QueryOptions,
  ) -> Result<Box<dyn Query>> {
    use tantivy::query::{BooleanQuery, QueryParser, RangeQuery};
    let mut query_parser = QueryParser::for_index(
      &self.index,
      vec![self.fields.title, self.fields.text],
    );

    if options.fuzzy {
      query_parser.set_field_fuzzy(self.fields.title, true, 1, true);
      query_parser.set_field_fuzzy(self.fields.text, true, 1, true);
    }

    let query = query_parser.parse_query(query)?;

    let to_bound = |d| match d {
      Some(d) => Bound::Included(d),
      None => Bound::Unbounded,
    };
    let title_range_query: Box<dyn Query> = if options.date_specified() {
      Box::new(RangeQuery::new_date_bounds(
        "title_date".into(),
        to_bound(options.date_after),
        to_bound(options.date_before),
      ))
    } else {
      Box::new(AllQuery)
    };

    let query = BooleanQuery::intersection(vec![query, title_range_query]);

    Ok(Box::new(query))
  }

  fn search_order_by_score(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
  ) -> Result<(usize, Vec<DocAddress>)> {
    use tantivy::collector::{Count, TopDocs};
    let mut collector = MultiCollector::new();
    let top_docs =
      TopDocs::with_limit(options.count).and_offset(options.offset);
    let top_docs_handle = collector.add_collector(top_docs);
    let total_records_handle = collector.add_collector(Count);

    let mut fruits = searcher.search(query, &collector)?;
    let top_docs = top_docs_handle
      .extract(&mut fruits)
      .into_iter()
      .map(|(_score, doc_addr)| doc_addr)
      .collect();
    let total_records = total_records_handle.extract(&mut fruits);

    Ok((total_records, top_docs))
  }

  fn search_order_by_date(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
  ) -> Result<(usize, Vec<DocAddress>)> {
    use tantivy::collector::{Count, TopDocs};
    let mut collector = MultiCollector::new();
    let top_docs = TopDocs::with_limit(options.count)
      .and_offset(options.offset)
      .order_by_fast_field::<DateTime>("title_date", Order::Desc);
    let top_docs_handle = collector.add_collector(top_docs);
    let total_records_handle = collector.add_collector(Count);

    let mut fruits = searcher.search(query, &collector)?;
    let top_docs = top_docs_handle
      .extract(&mut fruits)
      .into_iter()
      .map(|(_score, doc_addr)| doc_addr)
      .collect();
    let total_records = total_records_handle.extract(&mut fruits);

    Ok((total_records, top_docs))
  }

  fn search(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
  ) -> Result<(usize, Vec<DocAddress>)> {
    if options.date_specified() {
      self.search_order_by_date(searcher, options, query)
    } else {
      self.search_order_by_score(searcher, options, query)
    }
  }

  fn generate_docs(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
    top_docs: Vec<DocAddress>,
  ) -> Result<Vec<PageMatchEntry>> {
    let title_snippet_gen =
      SnippetGenerator::create(searcher, query, self.fields.title)?;
    let mut text_snippet_gen =
      SnippetGenerator::create(searcher, query, self.fields.text)?;
    text_snippet_gen.set_max_num_chars(options.snippet_length);

    let mut entries = vec![];
    for addr in top_docs {
      let doc = searcher.doc(addr)?;
      let page_id = doc.get_first(self.fields.id).unwrap().as_i64().unwrap();
      let namespace = text_field(&doc, self.fields.namespace);

      let title = {
        let source = text_field(&doc, self.fields.title);
        let snippet = title_snippet_gen.snippet_from_doc(&doc);
        MatchSnippet::new(source, snippet, options.snippet_length)
      };
      let text = {
        let source = text_field(&doc, self.fields.text);
        let snippet = text_snippet_gen.snippet_from_doc(&doc);
        MatchSnippet::new(source, snippet, options.snippet_length)
      };
      let url = text_field(&doc, self.fields.url);

      entries.push(PageMatchEntry {
        namespace,
        title,
        text,
        url,
        page_id,
      });
    }

    Ok(entries)
  }

  pub fn query(
    &self,
    query: &str,
    options: &QueryOptions,
  ) -> Result<PageMatchResult> {
    let start = std::time::Instant::now();
    let mut searcher = self.index.reader()?.searcher();

    let query = self.parse_query(query, options)?;
    let (total_records, top_docs) =
      self.search(&mut searcher, options, &query)?;
    let entries =
      self.generate_docs(&mut searcher, options, &query, top_docs)?;
    let new_offset = options.offset + entries.len();
    let new_offset = if new_offset < total_records {
      Some(new_offset)
    } else {
      None
    };
    let remaining = total_records - options.offset;

    let elapsed = start.elapsed();
    Ok(PageMatchResult {
      remaining,
      new_offset,
      entries,
      elapsed,
    })
  }

  fn make_doc(&self, page: Page) -> Result<Document> {
    let mut doc = Document::new();
    let f = &self.fields;

    doc.add_i64(f.id, page.id);
    doc.add_text(f.title, page.title);
    doc.add_text(f.text, page.text);
    doc.add_text(f.url, page.url);

    if let Some(title_date) = page.title_date.timestamp() {
      let tantivy_date = DateTime::from_timestamp_secs(title_date);
      doc.add_date(f.title_date, tantivy_date);
    }

    let tantivy_date = DateTime::from_timestamp_secs(page.updated.timestamp());
    doc.add_date(f.updated, tantivy_date);
    doc.add_text(f.namespace, &page.namespace.to_string());

    for cat in page.categories.iter() {
      doc.add_text(f.category, cat);
    }

    Ok(doc)
  }

  pub fn page_count(&self) -> Result<usize> {
    let reader = self.index.reader()?;
    Ok(reader.searcher().num_docs() as usize)
  }
}

fn build_schema() -> (Fields, Schema) {
  use tantivy::schema::*;

  let mut schema_builder = Schema::builder();

  let text_opt = TextOptions::default().set_stored().set_indexing_options(
    TextFieldIndexing::default()
      .set_tokenizer("text")
      .set_index_option(IndexRecordOption::WithFreqsAndPositions),
  );

  let casei_opt = TextOptions::default()
    .set_stored()
    .set_fast(Some("casei"))
    .set_indexing_options(
      TextFieldIndexing::default()
        .set_tokenizer("casei")
        .set_index_option(IndexRecordOption::WithFreqs),
    );

  let id = schema_builder.add_i64_field("id", STORED | FAST);
  let title = schema_builder.add_text_field("title", text_opt.clone());
  let text = schema_builder.add_text_field("text", text_opt);
  let title_date = schema_builder.add_date_field("title_date", STORED | FAST);
  let updated = schema_builder.add_date_field("updated", STORED | FAST);
  let namespace = schema_builder.add_text_field("namespace", casei_opt.clone());
  let url = schema_builder.add_text_field("url", STORED | STRING);
  let category = schema_builder.add_text_field("category", casei_opt);

  let schema = schema_builder.build();

  let fields = Fields {
    id,
    title,
    text,
    title_date,
    updated,
    namespace,
    url,
    category,
  };

  (fields, schema)
}

fn text_tokenizer() -> TextAnalyzer {
  use tantivy::tokenizer::*;

  // base: tokenize Chinese words
  TextAnalyzer::builder(JiebaTokenizer)
    // lowercase all words
    .filter(LowerCaser)
    // stem english words
    .filter(Stemmer::new(Language::English))
    // normalize unicode punctuations
    .filter(AsciiFoldingFilter)
    // remove long tokens (e.g. base64)
    .filter(RemoveLongFilter::limit(32))
    .build()
}

fn casei_tokenizer() -> TextAnalyzer {
  use tantivy::tokenizer::*;

  // base: do not tokenize
  TextAnalyzer::builder(RawTokenizer::default())
    // lowercase all words
    .filter(LowerCaser)
    // stem english words
    .filter(Stemmer::new(Language::English))
    // normalize unicode punctuations
    .filter(AsciiFoldingFilter)
    .build()
}

#[cfg(test)]
mod test {
  use tantivy::tokenizer::TextAnalyzer;
  const LOJBAN_SAMPLE_TEXT: &str = concat!(
    "邏輯語（逻辑语：la .lojban.，",
    "英語：Lojban，/ˈloʒban/  ( 聆聽)），",
    "一種人工語言，是Loglan的後繼者，",
    "由邏輯語言群（Logical Language Group，LLG）",
    "在1987年開始發展而成[1]。"
  );

  #[test]
  fn test_text_tokenizer() {
    let tokenizer = super::text_tokenizer();

    assert_eq!(
      tokenize(tokenizer, LOJBAN_SAMPLE_TEXT),
      vec![
        "邏輯語",
        "(",
        "逻辑",
        "语",
        ":",
        "la",
        " ",
        ".",
        "lojban",
        ".",
        ",",
        "英語",
        ":",
        "lojban",
        ",",
        "/",
        "ˈ",
        "lo",
        "ʒ",
        "ban",
        "/",
        " ",
        " ",
        "(",
        " ",
        "聆",
        "聽",
        ")",
        ")",
        ",",
        "一種",
        "人工",
        "語言",
        ",",
        "是",
        "loglan",
        "的",
        "後",
        "繼者",
        ",",
        "由",
        "邏輯",
        "語言群",
        "(",
        "logic",
        " ",
        "languag",
        " ",
        "group",
        ",",
        "llg",
        ")",
        "在",
        "1987",
        "年",
        "開始",
        "發展",
        "而成",
        "[",
        "1",
        "]",
        "。"
      ]
    );
  }

  fn tokenize(tokenizer: impl Into<TextAnalyzer>, s: &str) -> Vec<String> {
    let mut analyzer = tokenizer.into();
    let mut stream = analyzer.token_stream(s);
    let mut tokens = Vec::new();

    while let Some(token) = stream.next() {
      tokens.push(token.text.to_string());
    }

    tokens
  }
}

// assuming ranges are sorted
fn collapse_overlapped_ranges(ranges: &[Range<usize>]) -> Vec<Range<usize>> {
  let mut result = Vec::new();
  let mut ranges_it = ranges.iter();

  let mut current = match ranges_it.next() {
    Some(range) => range.clone(),
    None => return result,
  };

  for range in ranges {
    if current.end > range.start {
      current = current.start..std::cmp::max(current.end, range.end);
    } else {
      result.push(current);
      current = range.clone();
    }
  }

  result.push(current);
  result
}

fn text_field(doc: &Document, field: Field) -> String {
  doc.get_first(field).unwrap().as_text().unwrap().to_string()
}
