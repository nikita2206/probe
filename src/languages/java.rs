use crate::debug_trace;
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
        debug_trace!(
            "Traversing children of {} node at byte {}:{} (line {}:{})",
            node.kind(),
            node.start_byte(),
            node.end_byte(),
            node.start_position().row + 1,
            node.end_position().row + 1
        );
        
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            let mut child_count = 0;
            loop {
                child_count += 1;
                debug_trace!("Processing child #{}: {}", child_count, cursor.node().kind());
                self.collect_chunks_recursively(cursor.node(), content, stack, chunks);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            debug_trace!("Finished traversing {} children of {} node", child_count, node.kind());
        } else {
            debug_trace!("No children found for {} node", node.kind());
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
        debug_trace!(
            "Processing {} node at byte {}:{} (line {}:{}) - stack depth: {}",
            node.kind(),
            node.start_byte(),
            node.end_byte(),
            node.start_position().row + 1,
            node.end_position().row + 1,
            stack.len()
        );
        
        match node.kind() {
            "class_declaration" | "interface_declaration" | "record_declaration" => {
                let container_name = self.get_container_name(node, content);
                debug_trace!(
                    "Found {} '{}' - pushing to stack (current stack: [{}])",
                    node.kind(),
                    container_name,
                    stack.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>().join(", ")
                );
                stack.push((node, container_name.clone()));

                // Process children
                self.traverse_children(node, content, stack, chunks);

                debug_trace!("Popping '{}' from stack", container_name);
                stack.pop();
            }
            "method_declaration" => {
                // Skip lambdas/anonymous methods that don't have identifiable names
                if let Some(method_name) = self.get_method_name(node, content) {
                    debug_trace!(
                        "Found method '{}' in context: {}",
                        method_name,
                        if stack.is_empty() {
                            "global".to_string()
                        } else {
                            stack.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>().join("::")
                        }
                    );
                    
                    let (declaration, body) =
                        self.extract_method_with_context(node, content, stack);

                    let chunk = CodeChunk {
                        start_line: node.start_position().row,
                        end_line: node.end_position().row,
                        chunk_type: ChunkType::Method,
                        name: method_name.clone(),
                        content: body,
                        declaration,
                    };
                    
                    debug_trace!(
                        "Created chunk for method '{}': lines {}:{}, {} chars",
                        method_name,
                        chunk.start_line + 1,
                        chunk.end_line + 1,
                        chunk.content.len()
                    );
                    
                    chunks.push(chunk);
                } else {
                    debug_trace!("Skipping unnamed method (lambda/anonymous) at line {}", node.start_position().row + 1);
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
        debug_trace!("Getting container name for {} node", container_node.kind());
        
        let mut cursor = container_node.walk();
        if let Some(identifier_node) = utils::find_child_node(&mut cursor, &["identifier"]) {
            debug_trace!(
                "Found identifier node at byte {}:{} (line {}:{})",
                identifier_node.start_byte(),
                identifier_node.end_byte(),
                identifier_node.start_position().row + 1,
                identifier_node.end_position().row + 1
            );
            
            if let Ok(name) = identifier_node.utf8_text(content.as_bytes()) {
                debug_trace!("Extracted container name: '{}'", name);
                return name.to_string();
            } else {
                debug_trace!("Failed to extract UTF-8 text from identifier node");
            }
        } else {
            debug_trace!("No identifier child found for container");
        }
        debug_trace!("Using fallback name 'Anonymous'");
        "Anonymous".to_string()
    }

    fn get_method_name(&self, method_node: Node, content: &str) -> Option<String> {
        debug_trace!("Getting method name for method_declaration node");
        
        let mut cursor = method_node.walk();
        if let Some(identifier_node) = utils::find_child_node(&mut cursor, &["identifier"]) {
            debug_trace!(
                "Found method identifier at byte {}:{} (line {}:{})",
                identifier_node.start_byte(),
                identifier_node.end_byte(),
                identifier_node.start_position().row + 1,
                identifier_node.end_position().row + 1
            );
            
            if let Ok(name) = identifier_node.utf8_text(content.as_bytes()) {
                debug_trace!("Extracted method name: '{}'", name);
                return Some(name.to_string());
            } else {
                debug_trace!("Failed to extract UTF-8 text from method identifier");
            }
        } else {
            debug_trace!("No identifier child found for method declaration");
        }
        debug_trace!("Method has no identifiable name");
        None
    }

    /// Extracts the method with its full context from nested classes/interfaces
    fn extract_method_with_context(
        &self,
        method_node: Node,
        content: &str,
        stack: &[(Node, String)],
    ) -> (String, String) {
        debug_trace!(
            "Extracting method context - stack has {} containers",
            stack.len()
        );
        
        let mut declaration = String::new();

        // Build the full context from the stack
        for (i, (container_node, container_name)) in stack.iter().enumerate() {
            debug_trace!(
                "Processing stack entry #{}: {} '{}'",
                i,
                container_node.kind(),
                container_name
            );
            
            if i > 0 {
                declaration.push('\n');
            }

            // Add container declaration with proper indentation
            let (_container_start, start_node) =
                self.find_container_start_with_comments(*container_node);
            let container_body_start = self.find_container_body_start(*container_node);

            if let Some(body_start) = container_body_start {
                // Get line start index to include indentation in single substring
                let line_start_index = self.get_line_start_index(start_node);
                debug_trace!(
                    "Container '{}': line_start_index={}, body_start={}",
                    container_name,
                    line_start_index,
                    body_start
                );
                
                let container_with_indent = &content[line_start_index..body_start + 1];
                debug_trace!("Adding {} chars to declaration for container '{}'", container_with_indent.len(), container_name);
                declaration.push_str(container_with_indent);
            } else {
                debug_trace!("No body start found for container '{}'", container_name);
            }
        }

        // Add the method declaration
        debug_trace!("Splitting method declaration and body");
        let (method_declaration, method_body) =
            self.split_method_declaration_and_body(method_node, content);

        debug_trace!(
            "Method declaration: {} chars, body: {} chars",
            method_declaration.len(),
            method_body.len()
        );
        
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
        debug_trace!("Splitting method declaration and body");
        
        let body_node = utils::find_child_node(&mut method_node.walk(), &["block"]);

        match body_node {
            Some(body) => {
                debug_trace!(
                    "Found method body block at byte {}:{} (line {}:{})",
                    body.start_byte(),
                    body.end_byte(),
                    body.start_position().row + 1,
                    body.end_position().row + 1
                );
                
                // Extract declaration: from method start to body start + opening brace
                let method_start_with_indent =
                    method_node.start_byte() - method_node.start_position().column;
                let body_start = body.start_byte();
                
                debug_trace!(
                    "Method byte calculations: method_start={}, method_start_with_indent={}, body_start={}",
                    method_node.start_byte(),
                    method_start_with_indent,
                    body_start
                );
                
                let declaration = &content[method_start_with_indent..body_start + 1]; // +1 to include the opening brace

                // Extract body content (remove opening brace but keep closing brace)
                let body_content = if let Ok(body_text) = body.utf8_text(content.as_bytes()) {
                    debug_trace!("Extracted body text: {} chars", body_text.len());
                    let trimmed = body_text.trim();
                    if let Some(without_opening) = trimmed.strip_prefix('{') {
                        debug_trace!("Stripped opening brace from body");
                        // Remove the opening brace and any immediately following whitespace
                        without_opening.to_string()
                    } else {
                        debug_trace!("No opening brace found to strip");
                        trimmed.to_string()
                    }
                } else {
                    debug_trace!("Failed to extract UTF-8 text from body node");
                    String::new()
                };

                debug_trace!(
                    "Split result - declaration: {} chars, body: {} chars",
                    declaration.len(),
                    body_content.len()
                );

                (declaration.to_string(), body_content)
            }
            None => {
                debug_trace!("No body block found - likely abstract/interface method");
                // No body found (abstract method, interface method, etc.)
                let method_text = method_node.utf8_text(content.as_bytes()).unwrap_or("");
                debug_trace!("Using full method text: {} chars", method_text.len());
                (method_text.to_string(), String::new())
            }
        }
    }

    /// Finds the starting node and byte position of the class/interface declaration, including comments like JavaDocs.
    /// Returns (start_byte, start_node) where start_node is the earliest node (comment or declaration).
    fn find_container_start_with_comments<'a>(
        &self,
        container_node: Node<'a>,
    ) -> (usize, Node<'a>) {
        debug_trace!(
            "Finding container start with comments for {} at byte {}",
            container_node.kind(),
            container_node.start_byte()
        );
        
        let mut current = container_node;
        let mut start_byte = container_node.start_byte();
        let mut start_node = container_node;
        let mut comment_count = 0;

        // Look for preceding comment nodes (JavaDoc, block comments, line comments)
        while let Some(prev) = current.prev_sibling() {
            debug_trace!(
                "Checking previous sibling: {} at byte {}",
                prev.kind(),
                prev.start_byte()
            );
            
            match prev.kind() {
                "comment" | "block_comment" | "line_comment" => {
                    comment_count += 1;
                    debug_trace!("Found comment #{}: {} at byte {}", comment_count, prev.kind(), prev.start_byte());
                    start_byte = prev.start_byte();
                    start_node = prev;
                    current = prev;
                }
                _ => {
                    debug_trace!("Hit non-comment node: {} - stopping", prev.kind());
                    break; // Stop if we hit a non-comment node
                }
            }
        }

        debug_trace!(
            "Container start found: byte {} (found {} preceding comments)",
            start_byte,
            comment_count
        );
        
        (start_byte, start_node)
    }

    /// Finds the index of the first character of the class/interface body, which you can also think of as the end of the declaration block.
    fn find_container_body_start(&self, container_node: Node) -> Option<usize> {
        debug_trace!(
            "Finding body start for {} at byte {}",
            container_node.kind(),
            container_node.start_byte()
        );
        
        // In Java class/interface declarations, the body is typically the last child
        // Let's check children more efficiently
        let mut cursor = container_node.walk();
        if let Some(body_node) = utils::find_child_node(
            &mut cursor,
            &["class_body", "interface_body", "record_body"],
        ) {
            let body_start = body_node.start_byte() + 1; // +1 to skip the opening brace
            debug_trace!(
                "Found {} at byte {} - body starts at {}",
                body_node.kind(),
                body_node.start_byte(),
                body_start
            );
            return Some(body_start);
        }
        
        debug_trace!("No body node found for container");
        None
    }

    /// Returns the byte index of the line start for the given node
    /// This allows extracting both indentation and content in a single substring operation
    fn get_line_start_index(&self, node: Node) -> usize {
        let start_byte = node.start_byte();
        let start_column = node.start_position().column;
        
        debug_trace!(
            "Calculating line start index for {} node: start_byte={}, start_column={}",
            node.kind(),
            start_byte,
            start_column
        );

        let line_start = if start_byte >= start_column {
            start_byte - start_column
        } else {
            debug_trace!("start_byte < start_column - using start_byte as fallback");
            start_byte
        };
        
        debug_trace!("Line start index calculated: {}", line_start);
        line_start
    }
}

impl LanguageProcessor for JavaProcessor {
    fn get_file_extensions(&self) -> &[&str] {
        &["java"]
    }

    fn chunk_code(&mut self, content: &str) -> Result<Vec<CodeChunk>> {
        debug_trace!("Starting Java code chunking for {} bytes", content.len());
        
        let tree = self
            .parser
            .parse(content, None)
            .context("Failed to parse Java file")?;
        let root_node = tree.root_node();

        debug_trace!(
            "Successfully parsed Java file - root node: {} at byte {}:{} (line {}:{})",
            root_node.kind(),
            root_node.start_byte(),
            root_node.end_byte(),
            root_node.start_position().row + 1,
            root_node.end_position().row + 1
        );

        let mut chunks = Vec::new();
        let mut stack = Vec::new();

        self.collect_chunks_recursively(root_node, content, &mut stack, &mut chunks);

        debug_trace!("Java chunking completed - extracted {} chunks", chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            debug_trace!(
                "Chunk #{}: {:?} '{}' lines {}:{}", 
                i + 1,
                chunk.chunk_type,
                chunk.name,
                chunk.start_line + 1,
                chunk.end_line + 1
            );
        }

        Ok(chunks)
    }

    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>> {
        Ok(Box::new(JavaProcessor::new()?))
    }
}
