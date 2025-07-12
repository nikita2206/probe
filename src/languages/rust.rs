use anyhow::{Context, Result};
use tree_sitter::{Parser, Query, QueryCursor};
use crate::language_processor::{LanguageProcessor, CodeChunk, ChunkType, utils};

pub struct RustProcessor {
    parser: Parser,
    query: Query,
}

impl RustProcessor {
    pub fn new() -> Result<Self> {
        let language = tree_sitter_rust::language();
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .context("Failed to set Rust language")?;
        
        let query = Query::new(
            language,
            r#"
            (impl_item 
                type: (type_identifier) @type_name
                body: (declaration_list
                    (function_item
                        name: (identifier) @name) @method))
            (source_file
                (function_item 
                    name: (identifier) @name) @function)
            (struct_item
                name: (type_identifier) @name) @struct
            (trait_item
                name: (type_identifier) @name) @trait
            (mod_item
                name: (identifier) @name) @module
        "#,
        )
        .context("Failed to create Rust query")?;

        Ok(Self {
            parser,
            query,
        })
    }
}

impl LanguageProcessor for RustProcessor {
    fn get_file_extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>> {
        let tree = self.parser
            .parse(content, None)
            .context("Failed to parse Rust file")?;
        let root_node = tree.root_node();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&self.query, root_node, content.as_bytes());

        let mut chunks = Vec::new();

        for match_ in matches {
            let mut chunk_node = None;
            let mut name = String::new();
            let mut chunk_type = ChunkType::Other;

            for capture in match_.captures {
                let capture_name = &self.query.capture_names()[capture.index as usize];
                let node = capture.node;

                match capture_name.as_str() {
                    "name" => {
                        name = node.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                    }
                    "function" => {
                        chunk_node = Some(node);
                        chunk_type = ChunkType::Function;
                    }
                    "method" => {
                        chunk_node = Some(node);
                        chunk_type = ChunkType::Method;
                    }
                    "struct" => {
                        chunk_node = Some(node);
                        chunk_type = ChunkType::Struct;
                    }
                    "trait" => {
                        chunk_node = Some(node);
                        chunk_type = ChunkType::Interface;
                    }
                    "module" => {
                        chunk_node = Some(node);
                        chunk_type = ChunkType::Module;
                    }
                    _ => {}
                }
            }

            if let Some(node) = chunk_node {
                let start_byte = node.start_byte();
                let end_byte = node.end_byte();
                let start_line = node.start_position().row;
                let end_line = node.end_position().row;

                let chunk_content = if matches!(
                    chunk_type,
                    ChunkType::Struct | ChunkType::Interface
                ) {
                    // For structs/traits, extract only declaration + fields, exclude methods
                    utils::extract_class_declaration_with_fields(
                        content, start_byte, end_byte,
                    )?
                } else {
                    // For methods/functions, include full content
                    content.get(start_byte..end_byte).unwrap_or("").to_string()
                };

                // Skip if content is mostly imports
                if !utils::is_mostly_imports(&chunk_content) {
                    chunks.push(CodeChunk {
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        chunk_type,
                        name,
                        content: chunk_content,
                    });
                }
            }
        }

        // Sort chunks by start position
        chunks.sort_by_key(|c| c.start_byte);

        Ok(chunks)
    }

    fn split_method_content(&self, content: &str, chunk_type: &ChunkType) -> (String, String) {
        match chunk_type {
            ChunkType::Function | ChunkType::Method => {
                utils::split_function_declaration_and_body(content)
            }
            ChunkType::Struct | ChunkType::Interface => {
                utils::split_class_declaration_and_body(content)
            }
            _ => {
                // For other chunk types, treat whole content as declaration
                (content.to_string(), String::new())
            }
        }
    }
} 