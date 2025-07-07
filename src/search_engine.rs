use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use crate::file_scanner::FileScanner;
use crate::search_index::{SearchIndex, SearchResult};
use crate::metadata::IndexMetadata;
use crate::config::Config;
use crate::reranker::{Reranker, RerankerConfig, RerankDocument};

pub struct SearchEngine {
    root_dir: PathBuf,
    index_dir: PathBuf,
    metadata_path: PathBuf,
    config: Config,
}

impl SearchEngine {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self> {
        let root_path = root_dir.as_ref().to_path_buf();
        let index_dir = root_path.join(".codesearch");
        let metadata_path = index_dir.join("metadata.bin");
        let config = Config::load_from_dir(&root_path)?;
        
        Ok(Self {
            root_dir: root_path,
            index_dir,
            metadata_path,
            config,
        })
    }
    
    pub fn ensure_index_updated(&self) -> Result<()> {
        let scanner = FileScanner::new(&self.root_dir);
        let files = scanner.scan_files()?;
        
        let mut metadata = IndexMetadata::load(&self.metadata_path)?;
        let changed_files = metadata.needs_reindex(&files)?;
        
        if !changed_files.is_empty() {
            println!("Indexing {} changed files...", changed_files.len());
            
            let language = self.config.get_language()?;
            let mut index = match SearchIndex::open(&self.index_dir, language, self.config.stemming.enabled) {
                Ok(index) => index,
                Err(_) => SearchIndex::new(&self.index_dir, language, self.config.stemming.enabled)?,
            };
            
            index.index_files(&changed_files)?;
            
            // Update metadata for all files
            for file in &files {
                metadata.update_file(file)?;
            }
            
            metadata.save(&self.metadata_path)?;
            println!("Index updated. {} files tracked.", files.len());
        }
        
        Ok(())
    }
    
    pub fn rebuild_index(&self) -> Result<()> {
        println!("Rebuilding index from scratch...");
        
        // Remove existing index directory if it exists to handle schema changes
        if self.index_dir.exists() {
            std::fs::remove_dir_all(&self.index_dir)?;
        }
        
        let scanner = FileScanner::new(&self.root_dir);
        let files = scanner.scan_files()?;
        
        let language = self.config.get_language()?;
        let mut index = SearchIndex::new(&self.index_dir, language, self.config.stemming.enabled)?;
        index.index_files(&files)?;
        
        let mut metadata = IndexMetadata::new();
        for file in &files {
            metadata.update_file(file)?;
        }
        metadata.save(&self.metadata_path)?;
        
        println!("Index rebuilt. {} files indexed.", files.len());
        Ok(())
    }
    
    
    pub fn search_with_reranker(&self, query: &str, limit: Option<usize>, filetype: Option<&str>, reranker_config: RerankerConfig) -> Result<Vec<SearchResult>> {
        let language = self.config.get_language()?;
        let mut index = SearchIndex::open(&self.index_dir, language, self.config.stemming.enabled)?;
        
        // Determine how many candidates to fetch
        let final_limit = limit.unwrap_or(5);
        let fetch_limit = if reranker_config.enabled {
            // Fetch at least the minimum candidates, but more if user wants more results
            std::cmp::max(reranker_config.min_candidates, final_limit * 2)
        } else {
            final_limit
        };
        
        // Get initial results from Tantivy
        let mut results = index.search(query, fetch_limit, filetype)?;
        
        // Apply reranking if enabled and we have enough results
        if reranker_config.enabled && results.len() >= 2 {
            // Initialize reranker
            let reranker = Reranker::new(reranker_config)?;
            
            // Convert SearchResults to RerankDocuments
            let rerank_docs: Vec<RerankDocument> = results.into_iter().map(|result| {
                let mut metadata = HashMap::new();
                metadata.insert("path".to_string(), result.path.to_string_lossy().to_string());
                if let Some(chunk_type) = &result.chunk_type {
                    metadata.insert("chunk_type".to_string(), chunk_type.clone());
                }
                if let Some(chunk_name) = &result.chunk_name {
                    metadata.insert("chunk_name".to_string(), chunk_name.clone());
                }
                if let Some(start_line) = result.start_line {
                    metadata.insert("start_line".to_string(), start_line.to_string());
                }
                if let Some(end_line) = result.end_line {
                    metadata.insert("end_line".to_string(), end_line.to_string());
                }
                
                RerankDocument {
                    content: result.snippet.clone(),
                    metadata,
                }
            }).collect();
            
            // Perform reranking
            let rerank_result = reranker.rerank(query, rerank_docs, Some(final_limit))?;
            
            // Convert back to SearchResults
            results = rerank_result.documents.into_iter().enumerate().map(|(i, doc)| {
                let rerank_score = rerank_result.rerank_scores.get(i).copied().unwrap_or(0.0);
                SearchResult {
                    path: PathBuf::from(doc.metadata.get("path").unwrap_or(&String::new())),
                    score: rerank_score, // Use rerank score instead of original score
                    snippet: doc.content,
                    chunk_type: doc.metadata.get("chunk_type").cloned(),
                    chunk_name: doc.metadata.get("chunk_name").cloned(),
                    start_line: doc.metadata.get("start_line").and_then(|s| s.parse().ok()),
                    end_line: doc.metadata.get("end_line").and_then(|s| s.parse().ok()),
                }
            }).collect();
        } else {
            // No reranking, just limit results
            results.truncate(final_limit);
        }
        
        Ok(results)
    }
    
    pub fn stats(&self) -> Result<()> {
        let metadata = IndexMetadata::load(&self.metadata_path)?;
        println!("Files in index: {}", metadata.file_count());
        println!("Index directory: {}", self.index_dir.display());
        Ok(())
    }
}