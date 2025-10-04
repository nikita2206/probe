use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IndexMetadata {
    files: HashMap<PathBuf, FileInfo>,
}

impl IndexMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        match fs::read(&path) {
            Ok(data) => Ok(bincode::deserialize(&data)?),
            Err(_) => Ok(Self::new()), // Return empty metadata if file doesn't exist
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let data = bincode::serialize(self)?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)?;
        Ok(())
    }

    pub fn needs_reindex(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut changed_files = Vec::new();

        for file in files {
            if self.file_changed(file)? {
                changed_files.push(file.clone());
            }
        }

        Ok(changed_files)
    }

    pub fn update_file(&mut self, path: &Path) -> Result<()> {
        let metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(_) => return Ok(()), // Skip files that no longer exist
        };

        let file_info = FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified()?,
        };

        self.files.insert(path.to_path_buf(), file_info);
        Ok(())
    }

    fn file_changed(&self, path: &Path) -> Result<bool> {
        let current_metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(_) => return Ok(true), // File doesn't exist, consider it changed
        };

        match self.files.get(path) {
            Some(cached_info) => Ok(cached_info.size != current_metadata.len()
                || cached_info.modified != current_metadata.modified()?),
            None => Ok(true), // File not in cache, needs indexing
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}
