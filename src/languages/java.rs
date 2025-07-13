use crate::language_processor::utils;
use crate::language_processor::{ChunkType, CodeChunk, LanguageProcessor};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser, TreeCursor};

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

        Self::traverse_for_methods(&mut cursor, &mut methods);

        methods
    }

    fn traverse_for_methods<'a>(cursor: &mut TreeCursor<'a>, methods: &mut Vec<Node<'a>>) {
        if cursor.node().kind() == "method_declaration" {
            methods.push(cursor.node());
        }

        if cursor.goto_first_child() {
            loop {
                Self::traverse_for_methods(cursor, methods);
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
        if let Some(identifier_node) = utils::find_child_node(&mut cursor, &["identifier"]) {
            if let Ok(name) = identifier_node.utf8_text(content.as_bytes()) {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Extracts the container (class or interface) with the method declaration and body.
    ///
    /// @example
    ///
    /// ```java
    /// public class MyClass {
    ///     public void method() {
    ///         methodBody();
    ///     }
    /// }
    /// ```
    ///
    /// is split into:
    ///
    /// ```java
    /// public class MyClass {
    ///     public void method() {
    /// ```
    /// and
    ///
    /// ```java
    ///         methodBody();
    ///     }
    /// }
    /// ```
    fn extract_container_with_method(
        &self,
        content: &str,
        container_node: Node,
        method_node: Node,
    ) -> (String, String) {
        // Find the container declaration start (including JavaDoc/comments before it)
        let container_start = self.find_container_start_with_comments(container_node);
        let container_body_start = self.find_container_body_start(container_node);

        if let Some(body_start) = container_body_start {
            let mut declaration = String::new();

            // Add everything from the container start (including JavaDoc) up to the opening brace
            let container_decl = &content[container_start..body_start];
            declaration.push_str(container_decl);
            declaration.push('\n');

            // Split method into declaration and body using AST
            let (method_declaration, method_body) =
                self.split_method_declaration_and_body(method_node, content);

            declaration.push_str(method_declaration.as_str());

            return (declaration.trim_end().to_string(), method_body);
        }

        (String::new(), String::new())
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

    /// Finds the index of the first character of the class/interface declaration, including comments like JavaDocs.
    fn find_container_start_with_comments(&self, container_node: Node) -> usize {
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

        // Find all method declarations
        let methods = self.find_all_methods(root_node);

        for method_node in methods {
            if let Some(method_name) = self.get_method_name(method_node, content) {
                // Find the enclosing class or interface
                if let Some(container_node) = self.find_enclosing_class_or_interface(method_node) {
                    let (declaration, body) =
                        self.extract_container_with_method(content, container_node, method_node);

                    chunks.push(CodeChunk {
                        start_line: method_node.start_position().row,
                        end_line: method_node.end_position().row,
                        chunk_type: ChunkType::Method,
                        name: method_name,
                        content: body,
                        declaration,
                    });
                }
            }
        }

        Ok(chunks)
    }

    fn clone_box(&self) -> Result<Box<dyn LanguageProcessor>> {
        Ok(Box::new(JavaProcessor::new()?))
    }
}
