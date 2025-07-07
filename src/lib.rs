pub mod file_scanner;
pub mod search_index;
pub mod metadata;
pub mod search_engine;
pub mod config;
pub mod code_chunker;
pub mod reranker;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_scanner_basic() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() {}").unwrap();
        
        let scanner = file_scanner::FileScanner::new(temp_dir.path());
        let files = scanner.scan_files().unwrap();
        
        assert!(!files.is_empty());
        assert!(files.contains(&test_file));
    }
    
    #[test]
    fn test_metadata_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let metadata_file = temp_dir.path().join("metadata.bin");
        
        fs::write(&test_file, "test content").unwrap();
        
        let mut metadata = metadata::IndexMetadata::new();
        metadata.update_file(&test_file).unwrap();
        metadata.save(&metadata_file).unwrap();
        
        let loaded_metadata = metadata::IndexMetadata::load(&metadata_file).unwrap();
        assert_eq!(loaded_metadata.file_count(), 1);
    }
}