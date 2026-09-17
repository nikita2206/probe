use crate::language_processor::{ChunkType, CodeChunk, LanguageProcessor};
use anyhow::Result;

/// Catch-all indexer for files that have no specialized language processor.
///
/// Specialized processors (currently Java) own their file extensions. Everything
/// else — Python, JavaScript, Markdown, YAML, Makefiles, extensionless text —
/// is indexed as a single whole-file chunk so it is still searchable.
pub struct FallbackProcessor;

impl FallbackProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FallbackProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageProcessor for FallbackProcessor {
    fn get_file_extensions(&self) -> &[&str] {
        // Empty: this processor is never selected by extension. CodeChunker
        // uses it only when no specialized processor claims the file.
        &[]
    }

    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>> {
        if content.trim().is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![CodeChunk {
            start_line: 0,
            end_line: content.lines().count().saturating_sub(1),
            chunk_type: ChunkType::Other,
            name: "file".to_string(),
            content: content.to_string(),
            declaration: String::new(),
        }])
    }

    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>> {
        Ok(Box::new(FallbackProcessor::new()))
    }
}
