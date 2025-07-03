use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::file_scanner::FileScanner;
use crate::search_index::{SearchIndex, SearchResult};
use crate::metadata::IndexMetadata;
use crate::config::Config;

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
    
    pub fn search(&self, query: &str, limit: Option<usize>, filetype: Option<&str>) -> Result<Vec<SearchResult>> {
        let language = self.config.get_language()?;
        let mut index = SearchIndex::open(&self.index_dir, language, self.config.stemming.enabled)?;
        let results = index.search(query, limit.unwrap_or(5), filetype)?;
        Ok(results)
    }
    
    pub fn stats(&self) -> Result<()> {
        let metadata = IndexMetadata::load(&self.metadata_path)?;
        println!("Files in index: {}", metadata.file_count());
        println!("Index directory: {}", self.index_dir.display());
        Ok(())
    }
}