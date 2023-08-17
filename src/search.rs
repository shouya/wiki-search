use std::{ops::Range, path::Path};

use clap::Args;
use serde::Deserialize;
use tantivy::{
  collector::MultiCollector,
  directory::MmapDirectory,
  query::Query,
  schema::{Field, Schema},
  tokenizer::TextAnalyzer,
  DocAddress, Document, Index, IndexWriter, Searcher, Snippet,
  SnippetGenerator,
};
use tantivy_jieba::JiebaTokenizer;

use crate::{page::Page, util::Result};

pub struct Fields {
  id: Field,
  title: Field,
  text: Field,
  title_date: Field,
  page_touched: Field,
  namespace: Field,
}

pub struct Search {
  #[allow(unused)]
  schema: Schema,
  fields: Fields,
  index: Index,
}

#[derive(Debug)]
pub struct PageMatchResult {
  pub entries: Vec<PageMatchEntry>,
  pub new_offset: Option<usize>,
  pub total_records: usize,
  pub elapsed: std::time::Duration,
}

#[derive(Debug)]
pub struct PageMatchEntry {
  pub namespace: String,
  pub title: MatchSnippet,
  pub text: MatchSnippet,
  pub page_id: i64,
  pub score: f32,
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

#[derive(Clone, Args, Deserialize)]
pub struct QueryOptions {
  /// offset of results
  #[clap(short('s'), long)]
  pub offset: Option<usize>,

  /// number of results
  #[clap(short('n'), long, default_value_t = 10)]
  pub count: usize,

  /// max length of snippet
  #[clap(short('l'), long, default_value_t = 400)]
  pub snippet_length: usize,
}

impl Default for QueryOptions {
  fn default() -> Self {
    QueryOptions {
      offset: None,
      count: 10,
      snippet_length: 100,
    }
  }
}

impl Search {
  pub fn new(index_dir: &Path) -> Result<Self> {
    let (fields, schema) = build_schema();
    let dir = MmapDirectory::open(index_dir)
      .map_err(|e| e.to_string())
      .unwrap();

    let index = Index::open_or_create(dir, schema.clone())?;
    index.tokenizers().register("text", text_tokenizer());

    Ok(Search {
      fields,
      schema,
      index,
    })
  }

  pub fn reindex_pages(
    &mut self,
    pages: impl IntoIterator<Item = Page>,
  ) -> Result<()> {
    let mut writer = self.index.writer(128_000_000)?;
    writer.delete_all_documents()?;
    self.index_pages_with(&writer, pages)?;
    writer.commit()?;
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

  fn parse_query(&self, query: &str) -> Result<Box<dyn Query>> {
    use tantivy::query::QueryParser;
    let query_parser = QueryParser::for_index(
      &self.index,
      vec![self.fields.title, self.fields.text],
    );

    Ok(query_parser.parse_query(query)?)
  }

  fn search(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
  ) -> Result<(usize, Vec<(f32, DocAddress)>)> {
    use tantivy::collector::{Count, TopDocs};
    let mut collector = MultiCollector::new();
    let top_docs_handle = collector.add_collector(
      TopDocs::with_limit(options.count)
        .and_offset(options.offset.unwrap_or(0)),
    );
    let total_records_handle = collector.add_collector(Count);

    let mut fruits = searcher.search(query, &collector)?;
    let top_docs = top_docs_handle.extract(&mut fruits);
    let total_records = total_records_handle.extract(&mut fruits);

    Ok((total_records, top_docs))
  }

  fn generate_docs(
    &self,
    searcher: &mut Searcher,
    options: &QueryOptions,
    query: &impl Query,
    top_docs: Vec<(f32, DocAddress)>,
  ) -> Result<Vec<PageMatchEntry>> {
    let title_snippet_gen =
      SnippetGenerator::create(searcher, query, self.fields.title)?;
    let mut text_snippet_gen =
      SnippetGenerator::create(searcher, query, self.fields.text)?;
    text_snippet_gen.set_max_num_chars(options.snippet_length);

    let mut entries = vec![];
    for (score, addr) in top_docs {
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

      entries.push(PageMatchEntry {
        namespace,
        title,
        text,
        page_id,
        score,
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

    let query = self.parse_query(query)?;
    let (total_records, top_docs) =
      self.search(&mut searcher, options, &query)?;
    let entries =
      self.generate_docs(&mut searcher, options, &query, top_docs)?;
    let new_offset = options.offset.unwrap_or(0) + entries.len();
    let new_offset = if new_offset < total_records {
      Some(new_offset)
    } else {
      None
    };

    let elapsed = start.elapsed();
    Ok(PageMatchResult {
      total_records,
      new_offset,
      entries,
      elapsed,
    })
  }

  fn make_doc(&self, page: Page) -> Result<Document> {
    use tantivy::DateTime;

    let mut doc = Document::new();
    let f = &self.fields;

    doc.add_i64(f.id, page.id);
    doc.add_text(f.title, page.title);
    doc.add_text(f.text, page.text);

    if let Some(title_date) = page.title_date.timestamp() {
      let tantivy_date = DateTime::from_timestamp_secs(title_date);
      doc.add_date(f.title_date, tantivy_date);
    }

    let tantivy_date =
      DateTime::from_timestamp_secs(page.page_touched.timestamp());
    doc.add_date(f.page_touched, tantivy_date);
    doc.add_text(f.namespace, &page.namespace.to_string());

    Ok(doc)
  }
}

fn build_schema() -> (Fields, Schema) {
  use tantivy::schema::*;

  let mut schema_builder = Schema::builder();

  let text_index_options =
    TextOptions::default().set_stored().set_indexing_options(
      TextFieldIndexing::default()
        .set_tokenizer("text")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

  let id = schema_builder.add_i64_field("id", STORED | FAST);
  let title =
    schema_builder.add_text_field("title", text_index_options.clone());
  let text = schema_builder.add_text_field("text", text_index_options);
  let title_date = schema_builder.add_date_field("title_date", STORED | FAST);
  let page_touched =
    schema_builder.add_date_field("page_touched", STORED | FAST);
  let namespace = schema_builder.add_text_field("namespace", STORED | STRING);

  let schema = schema_builder.build();

  let fields = Fields {
    id,
    title,
    text,
    title_date,
    page_touched,
    namespace,
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
