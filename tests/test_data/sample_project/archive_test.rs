// This is a test file to demonstrate snippet extraction issues
// The file starts with many lines that should NOT appear in the snippet
// when searching for terms that appear later in the file.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// This is the beginning of the file with lots of content
/// that should not be shown in the snippet when searching
/// for terms that appear much later in the file.
pub struct DataProcessor {
    cache: HashMap<String, String>,
    config: ProcessorConfig,
}

pub struct ProcessorConfig {
    max_size: usize,
    timeout: u64,
    debug_mode: bool,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            config: ProcessorConfig {
                max_size: 1024,
                timeout: 30,
                debug_mode: false,
            },
        }
    }

    pub fn process_file(&mut self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        self.cache.insert(path.to_string_lossy().to_string(), content.clone());
        Ok(content)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    // This function contains the search term "archive lc" that we want to find
    // It should appear in the snippet, not the beginning of the file
    pub fn archive_local_cache(&mut self, archive_path: &str) -> Result<(), std::io::Error> {
        // When searching for "archive lc", this line should appear in the snippet
        println!("Archiving local cache to: {}", archive_path);
        
        // Create archive lc directory if it doesn't exist
        if let Some(parent) = Path::new(archive_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Serialize cache data to archive lc format
        let cache_data = serde_json::to_string_pretty(&self.cache)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        fs::write(archive_path, cache_data)?;
        println!("Successfully created archive lc at: {}", archive_path);
        
        Ok(())
    }

    pub fn restore_from_archive(&mut self, archive_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(archive_path)?;
        self.cache = serde_json::from_str(&content)?;
        println!("Restored cache from archive lc: {}", archive_path);
        Ok(())
    }
}

// More content to make the file longer and push the search term further down
impl Default for DataProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_archive_functionality() {
        let mut processor = DataProcessor::new();
        let temp_dir = tempdir().unwrap();
        let archive_path = temp_dir.path().join("test_archive.json");
        
        // Test archive lc functionality
        processor.cache.insert("test".to_string(), "data".to_string());
        processor.archive_local_cache(archive_path.to_str().unwrap()).unwrap();
        
        assert!(archive_path.exists());
    }
}