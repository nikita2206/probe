use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;
use tantivy::{
    schema::{Schema, STORED, TEXT, Value, TextFieldIndexing, IndexRecordOption, TextOptions},
    Index, IndexWriter, TantivyDocument,
    collector::TopDocs,
    query::{QueryParser, TermQuery, BooleanQuery, Occur},
    SnippetGenerator, Snippet,
    Term,
    tokenizer::{TextAnalyzer, RegexTokenizer, LowerCaser, RemoveLongFilter, Stemmer, Language},
};
use atty::Stream;

pub struct SearchIndex {
    index: Index,
    path_field: tantivy::schema::Field,
    content_field: tantivy::schema::Field,
    filetype_field: tantivy::schema::Field,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub score: f32,
    pub snippet: String,
}

impl SearchIndex {
    pub fn new<P: AsRef<Path>>(index_dir: P, language: Language, stemming_enabled: bool) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let path_field = schema_builder.add_text_field("path", STORED);
        
        // Create custom tokenizer for camel case splitting with optional stemming
        let camel_case_tokenizer = if stemming_enabled {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap()
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(language))
            .build()
        } else {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap()
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build()
        };
        
        // Configure content field with custom tokenizer
        let content_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("camel_case")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        
        let content_field_options = TextOptions::default()
            .set_indexing_options(content_field_indexing)
            .set_stored();
        
        let content_field = schema_builder.add_text_field("content", content_field_options);
        let filetype_field = schema_builder.add_text_field("filetype", TEXT | STORED);
        let schema = schema_builder.build();
        
        fs::create_dir_all(&index_dir)?;
        let index = Index::create_in_dir(&index_dir, schema.clone())?;
        
        // Register the custom tokenizer
        index.tokenizers()
            .register("camel_case", camel_case_tokenizer);
        
        Ok(Self {
            index,
            path_field,
            content_field,
            filetype_field,
        })
    }
    
    pub fn open<P: AsRef<Path>>(index_dir: P, language: Language, stemming_enabled: bool) -> Result<Self> {
        let index = Index::open_in_dir(&index_dir)?;
        let schema = index.schema();
        let path_field = schema.get_field("path")?;
        let content_field = schema.get_field("content")?;
        let filetype_field = schema.get_field("filetype")?;
        
        // Register the custom tokenizer for existing indexes
        let camel_case_tokenizer = if stemming_enabled {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap()
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(language))
            .build()
        } else {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap()
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build()
        };
        
        index.tokenizers()
            .register("camel_case", camel_case_tokenizer);
        
        Ok(Self {
            index,
            path_field,
            content_field,
            filetype_field,
        })
    }
    
    pub fn index_files(&mut self, files: &[PathBuf]) -> Result<()> {
        let mut index_writer: IndexWriter<TantivyDocument> = self.index.writer(50_000_000)?; // 50MB heap
        
        for file_path in files {
            self.index_file(&mut index_writer, file_path)?;
        }
        
        index_writer.commit()?;
        Ok(())
    }
    
    fn index_file(&self, writer: &mut IndexWriter<TantivyDocument>, file_path: &Path) -> Result<()> {
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(_) => return Ok(()) // Skip files we can't read as text
        };
        
        // Extract file extension
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        let mut doc = TantivyDocument::new();
        doc.add_text(self.path_field, file_path.to_string_lossy().as_ref());
        doc.add_text(self.content_field, &content);
        doc.add_text(self.filetype_field, extension);
        
        writer.add_document(doc)?;
        Ok(())
    }
    
    pub fn search(&self, query_str: &str, limit: usize, filetype: Option<&str>) -> Result<Vec<SearchResult>> {
        let reader = self.index
            .reader_builder()
            .try_into()?;
        
        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
        let content_query = query_parser.parse_query(query_str)?;
        
        // Build the final query with optional filetype filter
        let final_query: Box<dyn tantivy::query::Query> = if let Some(filetype) = filetype {
            let filetype_term = Term::from_field_text(self.filetype_field, filetype);
            let filetype_query = TermQuery::new(filetype_term, tantivy::schema::IndexRecordOption::Basic);
            
            
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, content_query),
                (Occur::Must, Box::new(filetype_query)),
            ]))
        } else {
            content_query
        };
        
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::new();
        
        // Create snippet generator for proper match detection - re-parse query for snippet
        let snippet_query = query_parser.parse_query(query_str)?;
        let snippet_generator = SnippetGenerator::create(&searcher, &*snippet_query, self.content_field)?;
        
        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            
            let path_text = retrieved_doc
                .get_first(self.path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            // Use Tantivy to find matches, then render ourselves
            let snippet = snippet_generator.snippet_from_doc(&retrieved_doc);
            let snippet_text = self.render_snippet_with_terminal_colors(&snippet);
            
            results.push(SearchResult {
                path: PathBuf::from(path_text),
                score,
                snippet: snippet_text,
            });
        }
        
        Ok(results)
    }
    
    fn render_snippet_with_terminal_colors(&self, snippet: &Snippet) -> String {
        let text = snippet.fragment();
        let highlighted_ranges = snippet.highlighted();
        
        // Debug: let's see what we get (remove this line for production)
        // eprintln!("Debug: text='{}', ranges={:?}", text, highlighted_ranges);
        
        // If no highlighting needed, return plain text
        if highlighted_ranges.is_empty() {
            return text.to_string();
        }
        
        // Check if we should use colors
        let use_colors = atty::is(Stream::Stdout);
        let (highlight_start, highlight_end) = if use_colors {
            ("\x1b[1;33m", "\x1b[0m") // Bold yellow
        } else {
            ("", "") // No highlighting when not in terminal
        };
        
        let mut result = String::new();
        let mut last_end = 0;
        
        // Sort ranges by start position to handle overlapping ranges
        let mut ranges: Vec<_> = highlighted_ranges.iter().cloned().collect();
        ranges.sort_by_key(|r| r.start);
        
        for range in ranges {
            // Add text before highlight
            if range.start > last_end {
                result.push_str(&text[last_end..range.start]);
            }
            
            // Add highlighted text
            result.push_str(highlight_start);
            result.push_str(&text[range.start..range.end]);
            result.push_str(highlight_end);
            
            last_end = range.end;
        }
        
        // Add remaining text after last highlight
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }
        
        result
    }
    
    
}