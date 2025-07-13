use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

// Re-export from language_processor for now to avoid circular imports
pub use crate::language_processor::{utils, ChunkType, CodeChunk, LanguageProcessor};

// Import Java language processor only
use crate::languages::java::JavaProcessor;

pub struct CodeChunker {
    processors: HashMap<String, Box<dyn LanguageProcessor>>,
}

impl CodeChunker {
    pub fn new() -> Result<Self> {
        let processors = vec![Box::new(JavaProcessor::new()?)]
            .into_iter()
            .flat_map(|processor| {
                processor
                    .get_file_extensions()
                    .iter()
                    .map(|ext| (ext.to_string(), processor.clone_box().unwrap()))
                    .collect::<Vec<_>>()
            })
            .collect();

        let chunker = Self { processors };

        Ok(chunker)
    }

    pub fn chunk_code_for_indexing(
        &mut self,
        file_path: &Path,
        content: &str,
    ) -> Result<Vec<CodeChunk>> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        if let Some(processor) = self.processors.get_mut(extension) {
            processor.chunk_code(content)
        } else if !content.trim().is_empty() {
            Ok(vec![CodeChunk {
                start_line: 0,
                end_line: content.lines().count().saturating_sub(1),
                chunk_type: ChunkType::Other,
                name: "file".to_string(),
                content: content.to_string(),
                declaration: "".to_string(),
            }])
        } else {
            Ok(vec![])
        }
    }
}

impl Default for CodeChunker {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
