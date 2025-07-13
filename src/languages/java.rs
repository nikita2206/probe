use anyhow::{Context, Result};
use tree_sitter::{Parser, Node, TreeCursor};
use crate::language_processor::{LanguageProcessor, CodeChunk, ChunkType};

pub struct JavaProcessor {
    parser: Parser,
}

impl JavaProcessor {
    pub fn new() -> Result<Self> {
        let language = tree_sitter_java::language();
        let mut parser = Parser::new();
        parser
            .set_language(language)
            .context("Failed to set Java language")?;

        Ok(Self { parser })
    }

    fn find_all_methods<'a>(&self, root_node: Node<'a>) -> Vec<Node<'a>> {
        let mut methods = Vec::new();
        let mut cursor = root_node.walk();
        
        self.traverse_for_methods(&mut cursor, &mut methods);
        
        methods
    }
    
    fn traverse_for_methods<'a>(&self, cursor: &mut TreeCursor<'a>, methods: &mut Vec<Node<'a>>) {
        if cursor.node().kind() == "method_declaration" {
            methods.push(cursor.node());
        }
        
        if cursor.goto_first_child() {
            loop {
                self.traverse_for_methods(cursor, methods);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    
    fn find_enclosing_class_or_interface<'a>(&self, method_node: Node<'a>) -> Option<Node<'a>> {
        let mut current = method_node;
        
        while let Some(parent) = current.parent() {
            if parent.kind() == "class_declaration" || parent.kind() == "interface_declaration" {
                return Some(parent);
            }
            current = parent;
        }
        
        None
    }
    
    fn get_method_name(&self, method_node: Node, content: &str) -> Option<String> {
        // In Java method declarations, the method name is a direct child identifier
        // We can find it by looking for the identifier child that comes after modifiers/return type
        let mut cursor = method_node.walk();
        
        if cursor.goto_first_child() {
            // Skip non-identifier nodes (modifiers, return type, etc.)
            loop {
                let node = cursor.node();
                if node.kind() == "identifier" {
                    // Found the method name identifier
                    if let Ok(name) = node.utf8_text(content.as_bytes()) {
                        return Some(name.to_string());
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        None
    }
    
    fn extract_container_with_method(&self, content: &str, container_node: Node, method_node: Node) -> String {
        let mut result = String::new();
        
        // Find the container declaration start (including JavaDoc/comments before it)
        let container_start = self.find_container_start_with_comments(content, container_node);
        let container_body_start = self.find_container_body_start(container_node);
        
        if let Some(body_start) = container_body_start {
            // Add everything from the container start (including JavaDoc) up to the opening brace
            let container_decl = &content[container_start..body_start - 1]; // -1 to exclude the opening brace
            result.push_str(container_decl);
            result.push_str("{\n");
            
            // Add placeholder for other methods
            result.push_str("    // ...\n");
            
            // Add the specific method
            let method_start = method_node.start_byte();
            let method_end = method_node.end_byte();
            let method_content = &content[method_start..method_end];
            
            // Add method with proper indentation - preserve original indentation structure
            for line in method_content.lines() {
                if !line.trim().is_empty() {
                    // Add base class indentation (4 spaces) plus preserve relative indentation
                    let trimmed_line = line.trim_start();
                    let original_indent = line.len() - trimmed_line.len();
                    let method_base_indent = method_node.start_position().column;
                    let class_base_indent = 4; // Standard indentation for class members
                    
                    // Calculate relative indentation from the method's base indentation
                    let relative_indent = if original_indent > method_base_indent {
                        original_indent - method_base_indent
                    } else {
                        0
                    };
                    
                    let total_indent = class_base_indent + relative_indent;
                    result.push_str(&" ".repeat(total_indent));
                    result.push_str(trimmed_line);
                } else {
                    result.push_str(line);
                }
                result.push('\n');
            }
        }
        
        result.trim_end().to_string()
    }
    
    fn find_container_start_with_comments(&self, _content: &str, container_node: Node) -> usize {
        let mut current = container_node;
        let mut start_byte = container_node.start_byte();
        
        // Look for preceding comment nodes (JavaDoc, block comments, line comments)
        while let Some(prev) = current.prev_sibling() {
            match prev.kind() {
                "comment" | "block_comment" | "line_comment" => {
                    start_byte = prev.start_byte();
                    current = prev;
                }
                _ => break, // Stop if we hit a non-comment node
            }
        }
        
        start_byte
    }
    
    fn find_container_body_start(&self, container_node: Node) -> Option<usize> {
        // In Java class/interface declarations, the body is typically the last child
        // Let's check children more efficiently
        let mut cursor = container_node.walk();
        
        if cursor.goto_first_child() {
            loop {
                let node = cursor.node();
                if node.kind() == "class_body" || node.kind() == "interface_body" {
                    return Some(node.start_byte() + 1); // +1 to skip the opening brace
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        None
    }
}

impl LanguageProcessor for JavaProcessor {
    fn get_file_extensions(&self) -> &[&str] {
        &["java"]
    }

    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>> {
        let tree = self.parser
            .parse(content, None)
            .context("Failed to parse Java file")?;
        let root_node = tree.root_node();

        let mut chunks = Vec::new();
        
        // Find all method declarations
        let methods = self.find_all_methods(root_node);
        
        for method_node in methods {
            if let Some(method_name) = self.get_method_name(method_node, content) {
                // Find the enclosing class or interface
                if let Some(container_node) = self.find_enclosing_class_or_interface(method_node) {
                    let chunk_content = self.extract_container_with_method(content, container_node, method_node);
                    
                    chunks.push(CodeChunk {
                        start_byte: method_node.start_byte(),
                        end_byte: method_node.end_byte(),
                        start_line: method_node.start_position().row,
                        end_line: method_node.end_position().row,
                        chunk_type: ChunkType::Method,
                        name: method_name,
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
            ChunkType::Method => {
                // For our Java method chunks, we need to split at the method body
                // The content includes class declaration + method, so find where method body starts
                let lines: Vec<&str> = content.lines().collect();
                let mut declaration_lines = Vec::new();
                let mut body_lines = Vec::new();
                let mut found_method_body = false;
                
                for line in lines {
                    if line.trim() == "// ..." {
                        declaration_lines.push(line);
                        continue;
                    }
                    
                    if !found_method_body {
                        declaration_lines.push(line);
                        // Look for opening brace that starts method body (after method signature)
                        if line.trim_start().contains("(") && line.trim_start().contains(")") && line.contains('{') {
                            // This line contains method signature and opening brace
                            found_method_body = true;
                        }
                    } else {
                        body_lines.push(line);
                    }
                }
                
                (declaration_lines.join("\n"), body_lines.join("\n"))
            }
            _ => (content.to_string(), String::new())
        }
          }
  }  