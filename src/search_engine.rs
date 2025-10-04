use crate::config::Config;
use crate::file_scanner::FileScanner;
use crate::metadata::IndexMetadata;
use crate::reranker::{RerankDocument, Reranker, RerankerConfig};
use crate::search_index::{SearchIndex, SearchResult};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct SearchEngine {
    root_dir: PathBuf,
    index_dir: PathBuf,
    metadata_path: PathBuf,
    config: Config,
}

impl SearchEngine {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self> {
        let root_path = root_dir.as_ref().to_path_buf();
        let index_dir = root_path.join(".probe");
        let metadata_path = index_dir.join("metadata.bin");
        let config = Config::load_from_dir(&root_path)?;

        Ok(Self {
            root_dir: root_path,
            index_dir,
            metadata_path,
            config,
        })
    }

    pub fn ensure_index_updated(&self, verbose: bool) -> Result<()> {
        let scanner = FileScanner::new(&self.root_dir);
        let files_iter = scanner.iter_files();
        let files: Vec<_> = files_iter.collect::<Vec<_>>();

        if verbose {
            println!("[VERBOSE] Found {} files to check", files.len());
        }

        let mut metadata = IndexMetadata::load(&self.metadata_path)?;
        
        if verbose {
            println!("[VERBOSE] Loaded metadata with {} files", metadata.file_count());
        }
        
        let changed_files = metadata.needs_reindex(&files, verbose)?;

        if !changed_files.is_empty() {
            println!("Indexing {} changed files...", changed_files.len());

            if verbose {
                println!("[VERBOSE] Files that need reindexing:");
                for file in &changed_files {
                    println!("[VERBOSE]   - {}", file.display());
                }
            }

            let language = self.config.get_language()?;
            let mut index =
                match SearchIndex::open(&self.index_dir, language, self.config.stemming.enabled) {
                    Ok(index) => index,
                    Err(_) => {
                        SearchIndex::new(&self.index_dir, language, self.config.stemming.enabled)?
                    }
                };

            let indexed_files = index.index_files(changed_files.into_iter(), 8)?;

            // Update metadata for indexed files
            for file in indexed_files {
                metadata.update_file(&file)?;
            }

            // Update metadata for all files
            for file in &files {
                metadata.update_file(file)?;
            }

            metadata.save(&self.metadata_path)?;
            println!("Index updated. {} files tracked.", files.len());
        } else if verbose {
            println!("[VERBOSE] No files need reindexing");
        }

        Ok(())
    }

    pub fn rebuild_index(&self, verbose: bool) -> Result<()> {
        println!("Rebuilding index from scratch...");

        // Remove existing index directory if it exists to handle schema changes
        if self.index_dir.exists() {
            if verbose {
                println!("[VERBOSE] Removing existing index directory: {}", self.index_dir.display());
            }
            std::fs::remove_dir_all(&self.index_dir)?;
        }

        let scanner = FileScanner::new(&self.root_dir);
        let files_iter = scanner.iter_files();

        let language = self.config.get_language()?;
        let mut index = SearchIndex::new(&self.index_dir, language, self.config.stemming.enabled)?;

        // Index the files and get back an iterator of processed files
        let indexed_files = index.index_files(files_iter, 8)?;

        // Update metadata for indexed files
        let mut metadata = IndexMetadata::new();
        let mut file_count = 0;
        for file in indexed_files {
            if verbose && file_count < 10 {
                println!("[VERBOSE] Saving metadata for: {}", file.display());
            }
            metadata.update_file(&file)?;
            file_count += 1;
        }
        
        if verbose {
            println!("[VERBOSE] Saving metadata to: {}", self.metadata_path.display());
            println!("[VERBOSE] Total files in metadata: {}", metadata.file_count());
        }
        
        metadata.save(&self.metadata_path)?;

        println!("Index rebuilt. {file_count} files indexed.");
        Ok(())
    }

    pub fn search_with_reranker(
        &self,
        query: &str,
        limit: Option<usize>,
        filetype: Option<&str>,
        reranker_config: RerankerConfig,
    ) -> Result<Vec<SearchResult>> {
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
            let mut reranker = Reranker::new(reranker_config)?;

            // Convert SearchResults to RerankDocuments
            let rerank_docs: Vec<RerankDocument> = results
                .into_iter()
                .map(|result| {
                    let mut metadata = HashMap::new();
                    metadata.insert(
                        "path".to_string(),
                        result.path.to_string_lossy().to_string(),
                    );
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
                })
                .collect();

            // Perform reranking
            let rerank_result = reranker.rerank(query, rerank_docs, Some(final_limit))?;

            // Convert back to SearchResults
            results = rerank_result
                .documents
                .into_iter()
                .enumerate()
                .map(|(i, doc)| {
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
                })
                .collect();
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
