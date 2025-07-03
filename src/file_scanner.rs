use std::path::{Path, PathBuf};
use anyhow::Result;
use ignore::WalkBuilder;

pub struct FileScanner {
    root_dir: PathBuf,
}

impl FileScanner {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }
    
    pub fn scan_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        let walker = WalkBuilder::new(&self.root_dir)
            .hidden(false) // Include hidden files by default
            .git_ignore(true) // Respect .gitignore
            .git_global(true) // Respect global gitignore
            .git_exclude(true) // Respect .git/info/exclude
            .filter_entry(|entry| {
                // Exclude .codesearch directory to avoid indexing our own files
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".codesearch" && entry.path().is_dir() {
                        return false;
                    }
                }
                true
            })
            .build();
            
        for result in walker {
            let entry = result?;
            let path = entry.path();
            
            if path.is_file() && self.should_index_file(path)? {
                files.push(path.to_path_buf());
            }
        }
        
        Ok(files)
    }
    
    fn should_index_file(&self, path: &Path) -> Result<bool> {
        // Skip binary files and very large files
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            
            // Skip common binary extensions
            if matches!(ext.as_str(), 
                "exe" | "dll" | "so" | "dylib" | "bin" | "obj" | "o" | 
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "ico" | "svg" |
                "mp3" | "mp4" | "avi" | "mkv" | "zip" | "tar" | "gz" | "pdf"
            ) {
                return Ok(false);
            }
        }
        
        // Skip files that are too large (>1MB)
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 1_000_000 {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}