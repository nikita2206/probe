use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

// Re-export from language_processor for now to avoid circular imports
pub use crate::language_processor::{LanguageProcessor, CodeChunk, ChunkType, utils};

// Import language processors directly
use crate::languages::rust::RustProcessor;
use crate::languages::python::PythonProcessor;
use crate::languages::javascript::JavaScriptProcessor;
use crate::languages::go::GoProcessor;
use crate::languages::c::CProcessor;
use crate::languages::java::JavaProcessor;
use crate::languages::csharp::CSharpProcessor;

pub struct CodeChunker {
    processors: HashMap<String, Box<dyn LanguageProcessor>>,
}

impl CodeChunker {
    pub fn new() -> Result<Self> {
        let mut chunker = Self {
            processors: HashMap::new(),
        };

        chunker.setup_language_processors()?;
        Ok(chunker)
    }

    fn setup_language_processors(&mut self) -> Result<()> {
        // Rust
        let rust_processor = RustProcessor::new()?;
        for ext in rust_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(RustProcessor::new()?));
        }

        // Python
        let python_processor = PythonProcessor::new()?;
        for ext in python_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(PythonProcessor::new()?));
        }

        // JavaScript/TypeScript
        let js_processor = JavaScriptProcessor::new()?;
        for ext in js_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(JavaScriptProcessor::new()?));
        }

        // Go
        let go_processor = GoProcessor::new()?;
        for ext in go_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(GoProcessor::new()?));
        }

        // C/C++
        let c_processor = CProcessor::new()?;
        for ext in c_processor.get_file_extensions() {
            if ext == &"cpp" || ext == &"cc" || ext == &"cxx" || ext == &"hpp" {
                // Use C++ processor for C++ files
                self.processors.insert(ext.to_string(), Box::new(CProcessor::new_cpp()?));
            } else {
                // Use C processor for C files
                self.processors.insert(ext.to_string(), Box::new(CProcessor::new()?));
            }
        }

        // Java
        let java_processor = JavaProcessor::new()?;
        for ext in java_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(JavaProcessor::new()?));
        }

        // C#
        let csharp_processor = CSharpProcessor::new()?;
        for ext in csharp_processor.get_file_extensions() {
            self.processors.insert(ext.to_string(), Box::new(CSharpProcessor::new()?));
        }

        Ok(())
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
        } else {
            // For unsupported file types, return the entire file as one chunk (excluding imports)
            let filtered_content = utils::filter_imports(content);
            if !filtered_content.trim().is_empty() {
                Ok(vec![CodeChunk {
                    start_byte: 0,
                    end_byte: content.len(),
                    start_line: 0,
                    end_line: content.lines().count().saturating_sub(1),
                    chunk_type: ChunkType::Other,
                    name: "file".to_string(),
                    content: filtered_content,
                }])
            } else {
                Ok(vec![])
            }
        }
    }

    pub fn split_method_content(&self, content: &str, chunk_type: &ChunkType) -> (String, String) {
        match chunk_type {
            ChunkType::Function | ChunkType::Method => {
                utils::split_function_declaration_and_body(content)
            }
            ChunkType::Class | ChunkType::Struct | ChunkType::Interface => {
                utils::split_class_declaration_and_body(content)
            }
            _ => {
                // For other chunk types, treat whole content as declaration
                (content.to_string(), String::new())
            }
        }
    }
}
