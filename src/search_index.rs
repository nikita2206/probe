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
use crate::code_chunker::{CodeChunker, ChunkType};

pub struct SearchIndex {
    index: Index,
    path_field: tantivy::schema::Field,
    content_field: tantivy::schema::Field,
    filetype_field: tantivy::schema::Field,
    code_chunker: CodeChunker,
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
        
        let code_chunker = CodeChunker::new()?;
        
        Ok(Self {
            index,
            path_field,
            content_field,
            filetype_field,
            code_chunker,
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
        
        let code_chunker = CodeChunker::new()?;
        
        Ok(Self {
            index,
            path_field,
            content_field,
            filetype_field,
            code_chunker,
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
    
    pub fn search(&mut self, query_str: &str, limit: usize, filetype: Option<&str>) -> Result<Vec<SearchResult>> {
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
            
            // Use code chunking for better snippets
            let snippet_text = self.generate_code_aware_snippet(&PathBuf::from(path_text), &retrieved_doc, &snippet_generator)
                .unwrap_or_else(|_| {
                    // Fallback to original snippet generation
                    let snippet = snippet_generator.snippet_from_doc(&retrieved_doc);
                    self.render_snippet_with_terminal_colors(&snippet)
                });
            
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
    
    fn generate_code_aware_snippet(
        &mut self,
        file_path: &Path,
        doc: &TantivyDocument,
        snippet_generator: &SnippetGenerator,
    ) -> Result<String> {
        let content = doc
            .get_first(self.content_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // Get code chunks
        let chunks = self.code_chunker.chunk_code(file_path, content)?;
        
        // If no chunks found (unsupported language), use original snippet
        if chunks.is_empty() || (chunks.len() == 1 && chunks[0].chunk_type == ChunkType::Other) {
            let snippet = snippet_generator.snippet_from_doc(doc);
            return Ok(self.render_snippet_with_terminal_colors(&snippet));
        }
        
        // Find chunks that contain matches, prioritizing methods over classes
        let mut matching_chunks = Vec::new();
        let mut seen_chunks = std::collections::HashSet::new();
        
        for chunk in chunks {
            // Skip if we've already seen this exact chunk (same name, type, and range)
            let chunk_key = (chunk.name.clone(), chunk.chunk_type.clone(), chunk.start_byte, chunk.end_byte);
            if seen_chunks.contains(&chunk_key) {
                continue;
            }
            
            // Create a temporary document with just this chunk's content
            let mut temp_doc = TantivyDocument::new();
            temp_doc.add_text(self.content_field, &chunk.content);
            
            // Generate snippet for this chunk
            let snippet = snippet_generator.snippet_from_doc(&temp_doc);
            
            // Check if this chunk has any highlighted text (i.e., contains matches)
            if !snippet.highlighted().is_empty() {
                matching_chunks.push((chunk, snippet));
                seen_chunks.insert(chunk_key);
            }
        }
        
        // If no matching chunks found, fallback to original snippet
        if matching_chunks.is_empty() {
            let snippet = snippet_generator.snippet_from_doc(doc);
            return Ok(self.render_snippet_with_terminal_colors(&snippet));
        }
        
        // Sort by priority: methods/functions first, then classes/structs
        matching_chunks.sort_by(|a, b| {
            let priority_a = match a.0.chunk_type {
                ChunkType::Method | ChunkType::Function => 0,
                ChunkType::Class | ChunkType::Struct | ChunkType::Interface => 1,
                ChunkType::Module => 2,
                ChunkType::Other => 3,
            };
            let priority_b = match b.0.chunk_type {
                ChunkType::Method | ChunkType::Function => 0,
                ChunkType::Class | ChunkType::Struct | ChunkType::Interface => 1,
                ChunkType::Module => 2,
                ChunkType::Other => 3,
            };
            priority_a.cmp(&priority_b).then_with(|| a.0.start_line.cmp(&b.0.start_line))
        });
        
        // Render matching chunks, showing full method bodies
        let mut result = String::new();
        let use_colors = atty::is(Stream::Stdout);
        let chunk_separator = if use_colors {
            "\n\x1b[36m───\x1b[0m\n" // Cyan separator
        } else {
            "\n---\n"
        };
        
        for (i, (chunk, _snippet)) in matching_chunks.iter().enumerate() {
            if i > 0 {
                result.push_str(chunk_separator);
            }
            
            // Add chunk header with name and type
            if use_colors {
                result.push_str(&format!("\x1b[32m{:?}\x1b[0m ", chunk.chunk_type));
                if !chunk.name.is_empty() {
                    result.push_str(&format!("\x1b[1m{}\x1b[0m", chunk.name));
                }
                result.push_str(&format!(" \x1b[90m(lines {}-{})\x1b[0m\n", 
                    chunk.start_line + 1, chunk.end_line + 1));
            } else {
                result.push_str(&format!("{:?} ", chunk.chunk_type));
                if !chunk.name.is_empty() {
                    result.push_str(&chunk.name);
                }
                result.push_str(&format!(" (lines {}-{})\n", 
                    chunk.start_line + 1, chunk.end_line + 1));
            }
            
            // For classes/structs/interfaces, show only declaration line; for methods/functions, show full body
            let content_to_show = match chunk.chunk_type {
                ChunkType::Class | ChunkType::Struct | ChunkType::Interface => {
                    self.extract_declaration_line(&chunk.content)
                },
                _ => chunk.content.clone()
            };
            
            let highlighted_content = self.highlight_content_matches(&content_to_show, snippet_generator)?;
            result.push_str(&highlighted_content);
        }
        
        Ok(result)
    }
    
    fn highlight_content_matches(&self, content: &str, snippet_generator: &SnippetGenerator) -> Result<String> {
        // Create a temporary document with the content
        let mut temp_doc = TantivyDocument::new();
        temp_doc.add_text(self.content_field, content);
        
        // Generate snippet to get highlight ranges
        let snippet = snippet_generator.snippet_from_doc(&temp_doc);
        let highlighted_ranges = snippet.highlighted();
        
        // If no highlighting needed, return plain text
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
        let mut ranges: Vec<_> = highlighted_ranges.iter().cloned().collect();
        ranges.sort_by_key(|r| r.start);
        
        for range in ranges {
            // Add text before highlight
            if range.start > last_end {
                result.push_str(&content[last_end..range.start]);
            }
            
            // Add highlighted text
            result.push_str(highlight_start);
            result.push_str(&content[range.start..range.end]);
            result.push_str(highlight_end);
            
            last_end = range.end;
        }
        
        // Add remaining text after last highlight
        if last_end < content.len() {
            result.push_str(&content[last_end..]);
        }
        
        Ok(result)
    }
    
    fn extract_declaration_line(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        
        // Find lines that likely contain class/struct/interface declarations
        let mut declaration_lines = Vec::new();
        let mut in_declaration = false;
        
        for line in &lines {
            let trimmed = line.trim();
            
            // Skip empty lines and comments at the start
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                if !in_declaration {
                    continue;
                }
            }
            
            // Check if this line contains class/struct/interface keywords or annotations
            if trimmed.starts_with('@') || // Java annotations
               trimmed.contains("class ") || 
               trimmed.contains("struct ") || 
               trimmed.contains("interface ") ||
               trimmed.contains("enum ") ||
               trimmed.contains("trait ") ||
               trimmed.contains("extends ") ||
               trimmed.contains("implements ") ||
               (in_declaration && (trimmed.contains("extends ") || trimmed.contains("implements ") || trimmed.ends_with('{'))) {
                
                declaration_lines.push(line);
                in_declaration = true;
                
                // Stop when we hit the opening brace (end of declaration)
                if trimmed.ends_with('{') {
                    break;
                }
            } else if in_declaration {
                // If we're in a declaration and hit a line that doesn't seem part of it, stop
                break;
            }
        }
        
        if declaration_lines.is_empty() {
            // Fallback: return first non-empty, non-comment line
            for line in &lines {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with("*") {
                    return line.to_string();
                }
            }
            // Ultimate fallback: return first few lines
            return lines.iter().take(3).map(|s| *s).collect::<Vec<_>>().join("\n");
        }
        
        declaration_lines.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n")
    }
}