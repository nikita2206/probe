use crate::language_processor::utils;
use crate::language_processor::{ChunkType, CodeChunk, LanguageProcessor};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

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

    /// Helper method to traverse all children of a node recursively.
    /// This avoids code duplication in the main traversal method.
    fn traverse_children<'a>(
        &self,
        node: Node<'a>,
        content: &str,
        stack: &mut Vec<(Node<'a>, String)>,
        chunks: &mut Vec<CodeChunk>,
    ) {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.collect_chunks_recursively(cursor.node(), content, stack, chunks);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    /// Recursively traverses the AST to collect code chunks.
    /// 
    /// The stack maintains the current nesting context of classes/interfaces
    /// to provide proper context for method declarations.
    /// 
    /// # Arguments
    /// * `stack` - Stack of (container_node, container_name) for nesting context
    fn collect_chunks_recursively<'a>(
        &self,
        node: Node<'a>,
        content: &str,
        stack: &mut Vec<(Node<'a>, String)>,
        chunks: &mut Vec<CodeChunk>,
    ) {
        match node.kind() {
            "class_declaration" | "interface_declaration" => {
                let container_name = self.get_container_name(node, content);
                stack.push((node, container_name));

                // Process children
                self.traverse_children(node, content, stack, chunks);

                stack.pop();
            }
            "method_declaration" => {
                // Skip lambdas/anonymous methods that don't have identifiable names
                if let Some(method_name) = self.get_method_name(node, content) {
                    let (declaration, body) =
                        self.extract_method_with_context(node, content, stack);

                    chunks.push(CodeChunk {
                        start_line: node.start_position().row,
                        end_line: node.end_position().row,
                        chunk_type: ChunkType::Method,
                        name: method_name,
                        content: body,
                        declaration,
                    });
                }
                // If method has no name (lambda/anonymous), we ignore it
            }
            _ => {
                // Process other nodes as needed
                self.traverse_children(node, content, stack, chunks);
            }
        }
    }

    fn get_container_name(&self, container_node: Node, content: &str) -> String {
        let mut cursor = container_node.walk();
        if let Some(identifier_node) = utils::find_child_node(&mut cursor, &["identifier"]) {
            if let Ok(name) = identifier_node.utf8_text(content.as_bytes()) {
                return name.to_string();
            }
        }
        "Anonymous".to_string()
    }

    fn get_method_name(&self, method_node: Node, content: &str) -> Option<String> {
        // In Java method declarations, the method name is a direct child identifier
        // We can find it by looking for the identifier child that comes after modifiers/return type
        let mut cursor = method_node.walk();
        if let Some(identifier_node) = utils::find_child_node(&mut cursor, &["identifier"]) {
            if let Ok(name) = identifier_node.utf8_text(content.as_bytes()) {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Extracts the method with its full context from nested classes/interfaces
    fn extract_method_with_context(
        &self,
        method_node: Node,
        content: &str,
        stack: &[(Node, String)],
    ) -> (String, String) {
        let mut declaration = String::new();

        // Build the full context from the stack
        for (i, (container_node, _container_name)) in stack.iter().enumerate() {
            if i > 0 {
                declaration.push('\n');
            }

            // Add container declaration with proper indentation
            let (container_start, start_node) = self.find_container_start_with_comments(*container_node);
            let container_body_start = self.find_container_body_start(*container_node);

            if let Some(body_start) = container_body_start {
                let container_decl = &content[container_start..body_start + 1];
                
                if i == 0 {
                    // First container - use original indentation as-is
                    declaration.push_str(container_decl);
                } else {
                    // Nested container - preserve relative indentation from source
                    let _start_node_column = start_node.start_position().column;
                    let source_indentation = self.extract_indentation_from_source(content, start_node);
                    
                    // Calculate additional indentation needed for nesting level
                    let additional_indent = self.calculate_nesting_indent(content, i);
                    
                    let adjusted_decl = self.adjust_container_indentation(
                        container_decl, 
                        &source_indentation, 
                        &additional_indent
                    );
                    declaration.push_str(&adjusted_decl);
                }
            }
        }

        // Add the method declaration
        let (method_declaration, method_body) =
            self.split_method_declaration_and_body(method_node, content);
        // Use the original indentation from the source code
        let method_declaration = method_declaration;

        if !declaration.is_empty() {
            declaration.push('\n');
        }
        declaration.push_str(&method_declaration);

        (declaration.trim_end().to_string(), method_body)
    }

    /// Splits the method declaration and body into two strings.
    ///
    /// @example
    ///
    /// ```java
    /// public void method() {
    ///     some.code();
    /// }
    /// ```
    /// is split into:
    ///
    /// ```java
    /// public void method() {
    /// ```
    /// and
    ///
    /// ```java
    ///     some.code();
    /// }
    /// ```
    fn split_method_declaration_and_body(
        &self,
        method_node: Node,
        content: &str,
    ) -> (String, String) {
        let body_node = utils::find_child_node(&mut method_node.walk(), &["block"]);

        match body_node {
            Some(body) => {
                // Extract declaration: from method start to body start + opening brace
                let method_start_with_indent =
                    method_node.start_byte() - method_node.start_position().column;
                let body_start = body.start_byte();
                let declaration = &content[method_start_with_indent..body_start + 1]; // +1 to include the opening brace

                // Extract body content (remove opening brace but keep closing brace)
                let body_content = if let Ok(body_text) = body.utf8_text(content.as_bytes()) {
                    let trimmed = body_text.trim();
                    if let Some(without_opening) = trimmed.strip_prefix('{') {
                        // Remove the opening brace and any immediately following whitespace
                        without_opening.to_string()
                    } else {
                        trimmed.to_string()
                    }
                } else {
                    String::new()
                };

                (declaration.to_string(), body_content)
            }
            None => {
                // No body found (abstract method, interface method, etc.)
                let method_text = method_node.utf8_text(content.as_bytes()).unwrap_or("");
                (method_text.to_string(), String::new())
            }
        }
    }

    /// Finds the starting node and byte position of the class/interface declaration, including comments like JavaDocs.
    /// Returns (start_byte, start_node) where start_node is the earliest node (comment or declaration).
    fn find_container_start_with_comments<'a>(&self, container_node: Node<'a>) -> (usize, Node<'a>) {
        let mut current = container_node;
        let mut start_byte = container_node.start_byte();
        let mut start_node = container_node;

        // Look for preceding comment nodes (JavaDoc, block comments, line comments)
        while let Some(prev) = current.prev_sibling() {
            match prev.kind() {
                "comment" | "block_comment" | "line_comment" => {
                    start_byte = prev.start_byte();
                    start_node = prev;
                    current = prev;
                }
                _ => break, // Stop if we hit a non-comment node
            }
        }

        (start_byte, start_node)
    }

    /// Finds the index of the first character of the class/interface body, which you can also think of as the end of the declaration block.
    fn find_container_body_start(&self, container_node: Node) -> Option<usize> {
        // In Java class/interface declarations, the body is typically the last child
        // Let's check children more efficiently
        let mut cursor = container_node.walk();
        if let Some(body_node) =
            utils::find_child_node(&mut cursor, &["class_body", "interface_body"])
        {
            return Some(body_node.start_byte() + 1); // +1 to skip the opening brace
        }
        None
    }

    /// Extracts the actual indentation string from the source code at the given node's line
    fn extract_indentation_from_source(&self, content: &str, node: Node) -> String {
        let start_row = node.start_position().row;
        let lines: Vec<&str> = content.lines().collect();
        
        if start_row < lines.len() {
            let line = lines[start_row];
            // Extract leading whitespace
            let indent_end = line.len() - line.trim_start().len();
            line[..indent_end].to_string()
        } else {
            String::new()
        }
    }

    /// Calculates the additional indentation needed based on nesting level
    /// This analyzes the source code's indentation pattern instead of assuming 4 spaces
    fn calculate_nesting_indent(&self, content: &str, nesting_level: usize) -> String {
        let indent_unit = self.detect_indentation_unit(content);
        indent_unit.repeat(nesting_level)
    }

    /// Detects the indentation unit used in the source code (spaces or tabs)
    fn detect_indentation_unit(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut space_indents = Vec::new();
        let mut has_tabs = false;

        for line in lines.iter().take(100) { // Sample first 100 lines
            if line.trim().is_empty() {
                continue;
            }
            
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            
            if indent_len > 0 {
                if line.starts_with('\t') {
                    has_tabs = true;
                    break;
                } else if line.starts_with(' ') {
                    space_indents.push(indent_len);
                }
            }
        }

        if has_tabs {
            "\t".to_string()
        } else if !space_indents.is_empty() {
            // Find the most common indentation level (likely the base unit)
            space_indents.sort_unstable();
            let mut min_indent = space_indents[0];
            
            // Find GCD-like pattern for indentation
            for &indent in &space_indents[1..] {
                if indent % min_indent != 0 {
                    // Find a smaller common unit
                    for i in (1..=min_indent).rev() {
                        if indent % i == 0 && min_indent % i == 0 {
                            min_indent = i;
                            break;
                        }
                    }
                }
            }
            
            " ".repeat(std::cmp::max(min_indent, 2)) // At least 2 spaces
        } else {
            "    ".to_string() // Default to 4 spaces
        }
    }

    /// Adjusts the indentation of a container declaration
    fn adjust_container_indentation(
        &self,
        container_decl: &str,
        _source_indentation: &str,
        additional_indent: &str,
    ) -> String {
        // Simply prepend the additional indentation to the container declaration
        // This preserves the original structure while adding nesting indentation
        format!("{}{}", additional_indent, container_decl)
    }
}

impl LanguageProcessor for JavaProcessor {
    fn get_file_extensions(&self) -> &[&str] {
        &["java"]
    }

    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>> {
        let tree = self
            .parser
            .parse(content, None)
            .context("Failed to parse Java file")?;
        let root_node = tree.root_node();

        let mut chunks = Vec::new();
        let mut stack = Vec::new();

        self.collect_chunks_recursively(root_node, content, &mut stack, &mut chunks);

        Ok(chunks)
    }

    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>> {
        Ok(Box::new(JavaProcessor::new()?))
    }
}
