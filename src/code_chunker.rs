use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

// Re-export from language_processor for now to avoid circular imports
pub use crate::language_processor::{utils, ChunkType, CodeChunk, LanguageProcessor};

use crate::languages::fallback::FallbackProcessor;
use crate::languages::java::JavaProcessor;

pub struct CodeChunker {
    processors: HashMap<String, Box<dyn LanguageProcessor>>,
    fallback: Box<dyn LanguageProcessor>,
}

impl CodeChunker {
    pub fn new() -> Result<Self> {
        let processors = vec![Box::new(JavaProcessor::new()?) as Box<dyn LanguageProcessor>]
            .into_iter()
            .flat_map(|processor| {
                processor
                    .get_file_extensions()
                    .iter()
                    .map(|ext| (ext.to_string(), processor.clone_box().unwrap()))
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(Self {
            processors,
            fallback: Box::new(FallbackProcessor::new()),
        })
    }

    /// True when a specialized (non-fallback) processor owns this file's extension.
    pub fn has_specialized_processor(&self, file_path: &Path) -> bool {
        Self::processor_key(file_path).is_some_and(|ext| self.processors.contains_key(ext))
    }

    pub fn chunk_code_for_indexing(
        &mut self,
        file_path: &Path,
        content: &str,
    ) -> Result<Vec<CodeChunk>> {
        if let Some(ext) = Self::processor_key(file_path) {
            if let Some(processor) = self.processors.get_mut(ext) {
                return processor.chunk_code(content);
            }
        }

        self.fallback.chunk_code(content)
    }

    fn processor_key(file_path: &Path) -> Option<&str> {
        file_path.extension().and_then(|ext| ext.to_str())
    }
}

impl Default for CodeChunker {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
