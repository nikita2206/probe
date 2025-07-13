use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_type: ChunkType,
    pub name: String,
    pub content: String,
    pub declaration: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChunkType {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Module,
    Other,
}

pub trait LanguageProcessor {
    /// Returns the file extensions this processor handles
    fn get_file_extensions(&self) -> &[&str];

    /// Chunks the given code content for indexing
    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>>;

    /// Creates a new boxed instance of this processor
    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>>;
}

/// Utility functions shared across language processors
pub mod utils {
    use tree_sitter::{Node, TreeCursor};

    /// Finds the first child node of the current node in the cursor that matches one of the provided kinds.
    /// Returns the node if found, or None otherwise.
    pub fn find_child_node<'a>(
        cursor: &mut TreeCursor<'a>,
        kinds: &[&str],
    ) -> Option<Node<'a>> {
        if cursor.goto_first_child() {
            loop {
                let node = cursor.node();
                if kinds.contains(&node.kind()) {
                    let found = node;
                    cursor.goto_parent(); // Restore cursor position
                    return Some(found);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent(); // Restore cursor position
        }
        None
    }
}
