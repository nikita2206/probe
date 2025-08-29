use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CodeChunk {
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

pub trait LanguageProcessor: Send + Sync {
    /// Returns the file extensions this processor handles
    fn get_file_extensions(&self) -> &[&str];

    /// Chunks the given code content for indexing
    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>>;

    /// Creates a new boxed instance of this processor
    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>>;
}

/// Utility functions shared across language processors
pub mod utils {
    use crate::debug_trace;
    use tree_sitter::{Node, TreeCursor};

    /// Finds the first child node of the current node in the cursor that matches one of the provided kinds.
    /// Returns the node if found, or None otherwise.
    pub fn find_child_node<'a>(cursor: &mut TreeCursor<'a>, kinds: &[&str]) -> Option<Node<'a>> {
        let parent_node = cursor.node();
        debug_trace!(
            "Searching for child nodes of type [{}] in {} node at byte {}",
            kinds.join(", "),
            parent_node.kind(),
            parent_node.start_byte()
        );
        
        if cursor.goto_first_child() {
            let mut child_count = 0;
            loop {
                child_count += 1;
                let node = cursor.node();
                debug_trace!(
                    "Checking child #{}: {} at byte {}:{}",
                    child_count,
                    node.kind(),
                    node.start_byte(),
                    node.end_byte()
                );
                
                if kinds.contains(&node.kind()) {
                    let found = node;
                    debug_trace!("Found matching child: {} at byte {}", found.kind(), found.start_byte());
                    cursor.goto_parent(); // Restore cursor position
                    return Some(found);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            debug_trace!("No matching children found after checking {} nodes", child_count);
            cursor.goto_parent(); // Restore cursor position
        } else {
            debug_trace!("No children to search in {} node", parent_node.kind());
        }
        None
    }
}
