use tantivy::{
  directory::MmapDirectory, schema::Schema, tokenizer::TextAnalyzer, Document,
  Index, IndexWriter,
};
use tantivy_jieba::JiebaTokenizer;

use crate::{config::Config, page::Page, util::Result};

pub struct Search {
  schema: Schema,
  index: Index,
}

impl Search {
  pub fn new(config: &Config) -> Result<Self> {
    let schema = build_schema();
    let dir = MmapDirectory::open(&config.index_dir)
      .map_err(|e| e.to_string())
      .unwrap();

    let index = Index::open_or_create(dir, schema.clone())?;
    index.tokenizers().register("text", text_tokenizer());

    Ok(Search { schema, index })
  }

  pub fn index_page(&self, writer: &IndexWriter, page: Page) -> Result<()> {
    writer.add_document(self.make_doc(page)?)?;

    Ok(())
  }

  pub fn index_pages(
    &mut self,
    pages: impl Iterator<Item = Page>,
  ) -> Result<()> {
    let mut writer = self.index.writer(50_000_000)?;

    for page in pages {
      self.index_page(&writer, page)?;
    }

    writer.commit()?;

    Ok(())
  }

  pub fn query(&self, query: &str) -> Result<Vec<(f32, i64)>> {
    use tantivy::collector::TopDocs;
    use tantivy::query::QueryParser;

    let id_field = self.schema.get_field("id").unwrap();

    let searcher = self.index.reader()?.searcher();
    println!("searching {} docs", searcher.num_docs());

    let query_parser = QueryParser::for_index(
      &self.index,
      vec![self.schema.get_field("text").unwrap()],
    );
    let query = query_parser.parse_query(query)?;
    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

    top_docs
      .into_iter()
      .map(|(score, addr)| {
        let doc = searcher.doc(addr)?;
        let id = doc.get_first(id_field).unwrap().as_i64().unwrap();
        Ok((score, id))
      })
      .collect()
  }

  fn make_doc(&self, page: Page) -> Result<Document> {
    use tantivy::DateTime;

    let mut doc = Document::new();
    let f = |name| self.schema.get_field(name).unwrap();

    doc.add_i64(f("id"), page.id as i64);
    doc.add_text(f("title"), page.title);
    doc.add_text(f("text"), page.text);

    if let Some(title_date) = page.title_date.timestamp() {
      let tantivy_date = DateTime::from_timestamp_secs(title_date);
      doc.add_date(f("title_date"), tantivy_date);
    }

    let tantivy_date =
      DateTime::from_timestamp_secs(page.page_touched.timestamp());
    doc.add_date(f("page_touched"), tantivy_date);
    doc.add_text(f("namespace"), &page.namespace.to_string());

    Ok(doc)
  }
}

fn build_schema() -> Schema {
  use tantivy::schema::*;

  let mut schema_builder = Schema::builder();

  let text_index_options =
    TextOptions::default().set_stored().set_indexing_options(
      TextFieldIndexing::default()
        .set_tokenizer("text")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

  schema_builder.add_i64_field("id", STORED | FAST);
  schema_builder.add_text_field("title", text_index_options.clone());
  schema_builder.add_text_field("text", text_index_options);
  schema_builder.add_date_field("title_date", STORED | FAST);
  schema_builder.add_date_field("page_touched", STORED | FAST);
  schema_builder.add_text_field("namespace", STORED | STRING);

  schema_builder.build()
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
  const LOJBAN_SAMPLE_TEXT: &str = "邏輯語（逻辑语：la .lojban.，英語：Lojban，/ˈloʒban/  ( 聆聽)），一種人工語言，是Loglan的後繼者，由邏輯語言群（Logical Language Group，LLG）在1987年開始發展而成[1]。";

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
