use crate::code_chunker::CodeChunker;
use anyhow::Result;
use atty::Stream;
use std::fs;
use std::path::{Path, PathBuf};
use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, TEXT},
    tokenizer::{Language, LowerCaser, RegexTokenizer, RemoveLongFilter, Stemmer, TextAnalyzer},
    Index, IndexWriter, Snippet, SnippetGenerator, TantivyDocument, Term,
};

pub struct SearchIndex {
    index: Index,
    path_field: tantivy::schema::Field,
    declaration_field: tantivy::schema::Field,
    body_field: tantivy::schema::Field,
    filetype_field: tantivy::schema::Field,
    chunk_type_field: tantivy::schema::Field,
    chunk_name_field: tantivy::schema::Field,
    start_line_field: tantivy::schema::Field,
    end_line_field: tantivy::schema::Field,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub score: f32,
    pub snippet: String,
    pub chunk_type: Option<String>,
    pub chunk_name: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

impl SearchIndex {
    pub fn new<P: AsRef<Path>>(
        index_dir: P,
        language: Language,
        stemming_enabled: bool,
    ) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let path_field = schema_builder.add_text_field("path", STORED);

        // Create custom tokenizer for camel case splitting with optional stemming
        let camel_case_tokenizer = if stemming_enabled {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap(),
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(language))
            .build()
        } else {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap(),
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build()
        };

        // Configure declaration and body fields with custom tokenizer
        let field_indexing = TextFieldIndexing::default()
            .set_tokenizer("camel_case")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);

        let field_options = TextOptions::default()
            .set_indexing_options(field_indexing)
            .set_stored();

        // Add declaration and body fields
        let declaration_field = schema_builder.add_text_field("declaration", field_options.clone());
        let body_field = schema_builder.add_text_field("body", field_options.clone());

        let filetype_field = schema_builder.add_text_field("filetype", TEXT | STORED);
        let chunk_type_field = schema_builder.add_text_field("chunk_type", TEXT | STORED);
        let chunk_name_field = schema_builder.add_text_field("chunk_name", TEXT | STORED);
        let start_line_field = schema_builder.add_u64_field("start_line", STORED);
        let end_line_field = schema_builder.add_u64_field("end_line", STORED);
        let schema = schema_builder.build();

        fs::create_dir_all(&index_dir)?;
        let index = Index::create_in_dir(&index_dir, schema.clone())?;

        // Register the custom tokenizer
        index
            .tokenizers()
            .register("camel_case", camel_case_tokenizer);

        Ok(Self {
            index,
            path_field,
            declaration_field,
            body_field,
            filetype_field,
            chunk_type_field,
            chunk_name_field,
            start_line_field,
            end_line_field,
        })
    }

    pub fn open<P: AsRef<Path>>(
        index_dir: P,
        language: Language,
        stemming_enabled: bool,
    ) -> Result<Self> {
        let index = Index::open_in_dir(&index_dir)?;
        let schema = index.schema();
        let path_field = schema.get_field("path")?;
        let declaration_field = schema.get_field("declaration")?;
        let body_field = schema.get_field("body")?;
        let filetype_field = schema.get_field("filetype")?;
        let chunk_type_field = schema.get_field("chunk_type").unwrap_or_else(|_| {
            // For backward compatibility with old indexes
            schema.get_field("filetype").unwrap()
        });
        let chunk_name_field = schema
            .get_field("chunk_name")
            .unwrap_or_else(|_| schema.get_field("filetype").unwrap());
        let start_line_field = schema
            .get_field("start_line")
            .unwrap_or_else(|_| schema.get_field("filetype").unwrap());
        let end_line_field = schema
            .get_field("end_line")
            .unwrap_or_else(|_| schema.get_field("filetype").unwrap());

        // Register the custom tokenizer for existing indexes
        let camel_case_tokenizer = if stemming_enabled {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap(),
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(language))
            .build()
        } else {
            TextAnalyzer::builder(
                RegexTokenizer::new(r"[a-z]+|[A-Z][a-z]*|[0-9]+|[^a-zA-Z0-9]+").unwrap(),
            )
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build()
        };

        index
            .tokenizers()
            .register("camel_case", camel_case_tokenizer);

        Ok(Self {
            index,
            path_field,
            declaration_field,
            body_field,
            filetype_field,
            chunk_type_field,
            chunk_name_field,
            start_line_field,
            end_line_field,
        })
    }

    pub fn index_files<I>(
        &mut self,
        files: I,
        num_threads: usize,
    ) -> Result<impl Iterator<Item = PathBuf>>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        use rayon::ThreadPoolBuilder;
        use std::sync::mpsc;

        let mut index_writer: IndexWriter<tantivy::TantivyDocument> =
            self.index.writer(50_000_000)?; // 50MB heap

        let (doc_tx, doc_rx) = mpsc::channel();
        let (path_tx, path_rx) = mpsc::channel();

        let files_vec: Vec<_> = files.into_iter().collect();

        // Only build global thread pool if it doesn't exist yet
        if ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .is_err()
        {
            // Global thread pool already exists, which is fine
        }

        rayon::scope(|s| {
            // Spawn worker threads to process files
            for file_path in &files_vec {
                let doc_tx = doc_tx.clone();
                let path_tx = path_tx.clone();
                let path_field = self.path_field;
                let declaration_field = self.declaration_field;
                let body_field = self.body_field;
                let filetype_field = self.filetype_field;
                let chunk_type_field = self.chunk_type_field;
                let chunk_name_field = self.chunk_name_field;
                let start_line_field = self.start_line_field;
                let end_line_field = self.end_line_field;
                let file_path = file_path.clone();
                s.spawn(move |_| {
                    // Create a new CodeChunker instance for this thread
                    let mut code_chunker = match CodeChunker::new() {
                        Ok(chunker) => chunker,
                        Err(_) => return,
                    };

                    let content = match fs::read_to_string(&file_path) {
                        Ok(content) => content,
                        Err(_) => return, // Skip files we can't read as text
                    };

                    // Skip files larger than 512KB or with lines longer than 8096 bytes
                    const MAX_FILE_SIZE: usize = 512 * 1024; // 512KB
                    const MAX_LINE_LENGTH: usize = 8096;

                    if content.len() > MAX_FILE_SIZE {
                        return; // Skip large files silently
                    }

                    if content.lines().any(|line| line.len() > MAX_LINE_LENGTH) {
                        return; // Skip files with very long lines silently
                    }
                    let extension = file_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    let chunks = match code_chunker.chunk_code_for_indexing(&file_path, &content) {
                        Ok(chunks) => chunks,
                        Err(_) => return,
                    };

                    // Send the file path to the caller
                    let _ = path_tx.send(file_path.clone());

                    if chunks.is_empty() {
                        let mut doc = tantivy::TantivyDocument::new();
                        doc.add_text(path_field, file_path.to_string_lossy().as_ref());
                        doc.add_text(declaration_field, "");
                        doc.add_text(body_field, &content);
                        doc.add_text(filetype_field, extension);
                        doc.add_text(chunk_type_field, "file");
                        doc.add_text(chunk_name_field, "");
                        doc.add_u64(start_line_field, 0);
                        doc.add_u64(
                            end_line_field,
                            content.lines().count().saturating_sub(1) as u64,
                        );
                        let _ = doc_tx.send(doc);
                    } else {
                        for chunk in chunks {
                            let mut doc = tantivy::TantivyDocument::new();
                            doc.add_text(path_field, file_path.to_string_lossy().as_ref());
                            doc.add_text(declaration_field, &chunk.declaration);
                            doc.add_text(body_field, &chunk.content);
                            doc.add_text(filetype_field, extension);
                            doc.add_text(chunk_type_field, format!("{:?}", chunk.chunk_type));
                            doc.add_text(chunk_name_field, &chunk.name);
                            doc.add_u64(start_line_field, chunk.start_line as u64);
                            doc.add_u64(end_line_field, chunk.end_line as u64);
                            let _ = doc_tx.send(doc);
                        }
                    }
                });
            }
        });
        drop(doc_tx); // Close the channel
        drop(path_tx); // Close the path channel

        // Process all documents from the channel
        for doc in doc_rx {
            index_writer.add_document(doc)?;
        }

        index_writer.commit()?;

        // Return an iterator over the processed file paths
        Ok(path_rx.into_iter())
    }

    pub fn search(
        &mut self,
        query_str: &str,
        limit: usize,
        filetype: Option<&str>,
        context_lines: usize,
    ) -> Result<Vec<SearchResult>> {
        let reader = self.index.reader_builder().try_into()?;

        let searcher = reader.searcher();

        // Create query parser with boosted fields - declaration gets higher boost than body
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.declaration_field,
                self.body_field,
                self.chunk_name_field,
            ],
        );

        // Set field boosts: declaration > chunk_name > body
        query_parser.set_field_boost(self.declaration_field, 3.0); // Highest boost for method declarations
        query_parser.set_field_boost(self.chunk_name_field, 2.5); // High boost for function/class names
        query_parser.set_field_boost(self.body_field, 1.0); // Baseline boost for method bodies

        let content_query = query_parser.parse_query(query_str)?;

        // Build the final query with optional filetype filter
        let final_query: Box<dyn tantivy::query::Query> = if let Some(filetype) = filetype {
            let filetype_term = Term::from_field_text(self.filetype_field, filetype);
            let filetype_query =
                TermQuery::new(filetype_term, tantivy::schema::IndexRecordOption::Basic);

            Box::new(BooleanQuery::new(vec![
                (Occur::Must, content_query),
                (Occur::Must, Box::new(filetype_query)),
            ]))
        } else {
            content_query
        };

        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit))?;
        let mut results = Vec::new();

        // Create snippet generators for both body and declaration fields
        let snippet_query = query_parser.parse_query(query_str)?;
        let snippet_generator =
            SnippetGenerator::create(&searcher, &*snippet_query, self.body_field)?;
        let declaration_snippet_generator =
            SnippetGenerator::create(&searcher, &*snippet_query, self.declaration_field)?;

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let path_text = retrieved_doc
                .get_first(self.path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Extract chunk metadata first
            let chunk_type = retrieved_doc
                .get_first(self.chunk_type_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Apply path-based and chunk-type score penalties
            let adjusted_score =
                Self::apply_score_penalties(score, path_text, chunk_type.as_deref());

            let chunk_name = retrieved_doc
                .get_first(self.chunk_name_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let start_line = retrieved_doc
                .get_first(self.start_line_field)
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            let end_line = retrieved_doc
                .get_first(self.end_line_field)
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            // Generate snippet with highlighting - for methods/functions, show full content
            let body_content = retrieved_doc
                .get_first(self.body_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let declaration_content = retrieved_doc
                .get_first(self.declaration_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let snippet_text = if matches!(chunk_type.as_deref(), Some("Function") | Some("Method"))
            {
                // For methods and functions, show the full content with highlighting
                if body_content.trim().is_empty() {
                    // For methods without bodies (e.g., interface methods), use declaration content
                    self.highlight_content(declaration_content, &declaration_snippet_generator)?
                } else {
                    // For methods with bodies, combine declaration and body for complete context
                    let declaration_highlighted = self
                        .highlight_content(declaration_content, &declaration_snippet_generator)?;
                    let body_highlighted =
                        self.highlight_content(body_content, &snippet_generator)?;
                    format!("{declaration_highlighted}{body_highlighted}")
                }
            } else if matches!(chunk_type.as_deref(), Some("Other") | Some("file")) {
                // For unsupported languages (entire files indexed), show relevant segments with context
                self.extract_relevant_segment_with_context(
                    body_content,
                    &snippet_generator,
                    context_lines,
                )?
            } else {
                // For other chunk types, use the default snippet behavior
                let snippet = snippet_generator.snippet_from_doc(&retrieved_doc);
                self.render_snippet_with_terminal_colors(&snippet)
            };

            results.push(SearchResult {
                path: PathBuf::from(path_text),
                score: adjusted_score,
                snippet: snippet_text,
                chunk_type,
                chunk_name,
                start_line,
                end_line,
            });
        }

        // Sort results by adjusted score in descending order (highest score first)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    fn apply_score_penalties(score: f32, path: &str, chunk_type: Option<&str>) -> f32 {
        let mut adjusted_score = score;

        // Apply penalty for test files - reduce score by 50% if path contains "test"
        if path.to_lowercase().contains("test") {
            adjusted_score *= 0.5;
        }

        // Apply penalty for Class chunks - down-rank them compared to Method chunks
        // Classes get 60% of their original score to prioritize methods in search results
        if matches!(
            chunk_type,
            Some("Class") | Some("Interface") | Some("Struct")
        ) {
            adjusted_score *= 0.6;
        }

        adjusted_score
    }

    fn render_snippet_with_terminal_colors(&self, snippet: &Snippet) -> String {
        let text = snippet.fragment();
        let highlighted_ranges = snippet.highlighted();

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
        let mut ranges: Vec<_> = highlighted_ranges.to_vec();
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

    fn highlight_content(
        &self,
        content: &str,
        snippet_generator: &SnippetGenerator,
    ) -> Result<String> {
        // Generate snippet to get highlight ranges
        let snippet = snippet_generator.snippet(content);
        let highlighted_ranges = snippet.highlighted();
        let highlight_fragment_offset = content.find(snippet.fragment()).unwrap_or(0);

        // If no highlighting needed, return the full content
        if highlighted_ranges.is_empty() {
            return Ok(content.to_string());
        }

        // Check if we should use colors
        let use_colors = atty::is(Stream::Stdout);
        let (highlight_start, highlight_end) = if use_colors {
            ("\x1b[1;33m", "\x1b[0m") // Bold yellow
        } else {
            ("", "")
        };

        let mut result = String::new();
        let mut last_end = 0;

        // Sort ranges by start position to handle overlapping ranges
        let mut ranges: Vec<_> = highlighted_ranges.to_vec();
        ranges.sort_by_key(|r| r.start);

        for range in ranges {
            // Add text before highlight
            if (range.start + highlight_fragment_offset) > last_end {
                result.push_str(&content[last_end..range.start + highlight_fragment_offset]);
            }

            // Add highlighted text
            result.push_str(highlight_start);
            result.push_str(
                &content[range.start + highlight_fragment_offset
                    ..range.end + highlight_fragment_offset],
            );
            result.push_str(highlight_end);

            last_end = range.end + highlight_fragment_offset;
        }

        // Add remaining text after last highlight
        if last_end < content.len() {
            result.push_str(&content[last_end..]);
        }

        Ok(result)
    }

    /// Extract the most relevant segment from a file with context lines around it.
    /// This is used for unsupported languages where entire files are indexed.
    fn extract_relevant_segment_with_context(
        &self,
        content: &str,
        snippet_generator: &SnippetGenerator,
        context_lines: usize,
    ) -> Result<String> {
        // Generate snippet to find the most relevant fragment
        let snippet = snippet_generator.snippet(content);
        let fragment = snippet.fragment();
        let highlighted_ranges = snippet.highlighted();

        // If no highlighting, just return the default snippet
        if highlighted_ranges.is_empty() {
            return Ok(self.render_snippet_with_terminal_colors(&snippet));
        }

        // Find where the fragment appears in the full content
        let fragment_offset = content.find(fragment).unwrap_or(0);

        // Convert content to lines for easier line-based extraction
        let lines: Vec<&str> = content.lines().collect();

        // Find which line contains the fragment
        let mut byte_count = 0;
        let mut fragment_line_idx = 0;
        for (idx, line) in lines.iter().enumerate() {
            let line_end = byte_count + line.len();
            if fragment_offset >= byte_count && fragment_offset < line_end {
                fragment_line_idx = idx;
                break;
            }
            byte_count = line_end + 1; // +1 for newline
        }

        // Calculate the range of lines to extract (with context)
        let start_line = fragment_line_idx.saturating_sub(context_lines);
        let end_line = (fragment_line_idx + context_lines + 1).min(lines.len());

        // Extract the segment
        let segment_lines = &lines[start_line..end_line];
        let segment = segment_lines.join("\n");

        // Calculate the offset adjustment for highlighting
        let mut segment_start_offset = 0;
        for line in &lines[0..start_line] {
            segment_start_offset += line.len() + 1; // +1 for newline
        }

        // Highlight the segment using the original highlight ranges
        let use_colors = atty::is(Stream::Stdout);
        let (highlight_start, highlight_end) = if use_colors {
            ("\x1b[1;33m", "\x1b[0m") // Bold yellow
        } else {
            ("", "")
        };

        let mut result = String::new();
        let mut last_end = 0;

        // Sort ranges by start position
        let mut ranges: Vec<_> = highlighted_ranges.to_vec();
        ranges.sort_by_key(|r| r.start);

        for range in ranges {
            let abs_start = fragment_offset + range.start;
            let abs_end = fragment_offset + range.end;

            // Skip ranges outside our segment
            if abs_end < segment_start_offset {
                continue;
            }
            if abs_start >= segment_start_offset + segment.len() {
                break;
            }

            // Adjust the range to be relative to our segment
            let seg_start = abs_start.saturating_sub(segment_start_offset);
            let seg_end = if abs_end > segment_start_offset {
                (abs_end - segment_start_offset).min(segment.len())
            } else {
                continue;
            };

            // Add text before highlight
            if seg_start > last_end {
                result.push_str(&segment[last_end..seg_start]);
            }

            // Add highlighted text
            result.push_str(highlight_start);
            result.push_str(&segment[seg_start..seg_end]);
            result.push_str(highlight_end);

            last_end = seg_end;
        }

        // Add remaining text after last highlight
        if last_end < segment.len() {
            result.push_str(&segment[last_end..]);
        }

        Ok(result)
    }
}
