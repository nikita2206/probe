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

    pub fn needs_reindex(&self, files: &[PathBuf], verbose: bool) -> Result<Vec<PathBuf>> {
        let mut changed_files = Vec::new();

        for file in files {
            let (changed, reason) = self.file_changed_with_reason(file)?;
            if changed {
                if verbose {
                    println!("[VERBOSE] File needs reindexing: {} - Reason: {}", file.display(), reason);
                }
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

    fn file_changed_with_reason(&self, path: &Path) -> Result<(bool, String)> {
        let current_metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(e) => return Ok((true, format!("File doesn't exist or can't be read: {}", e))),
        };

        match self.files.get(path) {
            Some(cached_info) => {
                if cached_info.size != current_metadata.len() {
                    Ok((true, format!("Size changed: {} -> {}", cached_info.size, current_metadata.len())))
                } else if cached_info.modified != current_metadata.modified()? {
                    Ok((true, format!("Modified time changed: {:?} -> {:?}", cached_info.modified, current_metadata.modified()?)))
                } else {
                    Ok((false, "No changes".to_string()))
                }
            }
            None => Ok((true, "File not in metadata cache".to_string())),
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}
