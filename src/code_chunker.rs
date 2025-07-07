use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, Context};
use tree_sitter::{Language, Parser, Query, QueryCursor};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CodeChunk {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_type: ChunkType,
    pub name: String,
    pub content: String,
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

pub struct CodeChunker {
    parsers: HashMap<String, (Language, Parser, Query)>,
}

impl CodeChunker {
    pub fn new() -> Result<Self> {
        let mut chunker = Self {
            parsers: HashMap::new(),
        };
        
        chunker.setup_language_parsers()?;
        Ok(chunker)
    }
    
    fn setup_language_parsers(&mut self) -> Result<()> {
        // Rust
        let rust_language = tree_sitter_rust::language();
        let mut rust_parser = Parser::new();
        rust_parser.set_language(rust_language).context("Failed to set Rust language")?;
        let rust_query = Query::new(rust_language, r#"
            (function_item 
                name: (identifier) @name) @function
            (impl_item 
                type: (type_identifier) @type_name
                body: (declaration_list
                    (function_item
                        name: (identifier) @name) @method))
            (struct_item
                name: (type_identifier) @name) @struct
            (trait_item
                name: (type_identifier) @name) @trait
            (mod_item
                name: (identifier) @name) @module
        "#).context("Failed to create Rust query")?;
        
        self.parsers.insert("rs".to_string(), (rust_language, rust_parser, rust_query));
        
        // Python
        let python_language = tree_sitter_python::language();
        let mut python_parser = Parser::new();
        python_parser.set_language(python_language).context("Failed to set Python language")?;
        let python_query = Query::new(python_language, r#"
            (function_definition
                name: (identifier) @name) @function
            (class_definition
                name: (identifier) @name) @class
        "#).context("Failed to create Python query")?;
        
        self.parsers.insert("py".to_string(), (python_language, python_parser, python_query));
        
        // JavaScript/TypeScript
        let js_language = tree_sitter_javascript::language();
        let mut js_parser = Parser::new();
        js_parser.set_language(js_language).context("Failed to set JavaScript language")?;
        let js_query = Query::new(js_language, r#"
            (function_declaration
                name: (identifier) @name) @function
            (method_definition
                name: (property_identifier) @name) @method
            (class_declaration
                name: (identifier) @name) @class
        "#).context("Failed to create JavaScript query")?;
        
        self.parsers.insert("js".to_string(), (js_language, js_parser, js_query));
        
        // TypeScript (same as JavaScript)
        let ts_language = tree_sitter_javascript::language();
        let mut ts_parser = Parser::new();
        ts_parser.set_language(ts_language).context("Failed to set TypeScript language")?;
        let ts_query = Query::new(ts_language, r#"
            (function_declaration
                name: (identifier) @name) @function
            (method_definition
                name: (property_identifier) @name) @method
            (class_declaration
                name: (identifier) @name) @class
        "#).context("Failed to create TypeScript query")?;
        self.parsers.insert("ts".to_string(), (ts_language, ts_parser, ts_query));
        
        // Go
        let go_language = tree_sitter_go::language();
        let mut go_parser = Parser::new();
        go_parser.set_language(go_language).context("Failed to set Go language")?;
        let go_query = Query::new(go_language, r#"
            (function_declaration
                name: (identifier) @name) @function
            (method_declaration
                name: (field_identifier) @name) @method
            (type_declaration
                (type_spec
                    name: (type_identifier) @name)) @type
        "#).context("Failed to create Go query")?;
        
        self.parsers.insert("go".to_string(), (go_language, go_parser, go_query));
        
        // C
        let c_language = tree_sitter_c::language();
        let mut c_parser = Parser::new();
        c_parser.set_language(c_language).context("Failed to set C language")?;
        let c_query = Query::new(c_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C query")?;
        
        self.parsers.insert("c".to_string(), (c_language, c_parser, c_query));
        
        // C headers
        let c_h_language = tree_sitter_c::language();
        let mut c_h_parser = Parser::new();
        c_h_parser.set_language(c_h_language).context("Failed to set C header language")?;
        let c_h_query = Query::new(c_h_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C header query")?;
        self.parsers.insert("h".to_string(), (c_h_language, c_h_parser, c_h_query));
        
        // C++
        let cpp_language = tree_sitter_cpp::language();
        let mut cpp_parser = Parser::new();
        cpp_parser.set_language(cpp_language).context("Failed to set C++ language")?;
        let cpp_query = Query::new(cpp_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (class_specifier
                name: (type_identifier) @name) @class
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C++ query")?;
        
        self.parsers.insert("cpp".to_string(), (cpp_language, cpp_parser, cpp_query));
        
        // C++ additional extensions
        let cpp_cc_language = tree_sitter_cpp::language();
        let mut cpp_cc_parser = Parser::new();
        cpp_cc_parser.set_language(cpp_cc_language).context("Failed to set C++ cc language")?;
        let cpp_cc_query = Query::new(cpp_cc_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (class_specifier
                name: (type_identifier) @name) @class
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C++ cc query")?;
        self.parsers.insert("cc".to_string(), (cpp_cc_language, cpp_cc_parser, cpp_cc_query));
        
        let cpp_cxx_language = tree_sitter_cpp::language();
        let mut cpp_cxx_parser = Parser::new();
        cpp_cxx_parser.set_language(cpp_cxx_language).context("Failed to set C++ cxx language")?;
        let cpp_cxx_query = Query::new(cpp_cxx_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (class_specifier
                name: (type_identifier) @name) @class
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C++ cxx query")?;
        self.parsers.insert("cxx".to_string(), (cpp_cxx_language, cpp_cxx_parser, cpp_cxx_query));
        
        let cpp_hpp_language = tree_sitter_cpp::language();
        let mut cpp_hpp_parser = Parser::new();
        cpp_hpp_parser.set_language(cpp_hpp_language).context("Failed to set C++ hpp language")?;
        let cpp_hpp_query = Query::new(cpp_hpp_language, r#"
            (function_definition
                declarator: (function_declarator
                    declarator: (identifier) @name)) @function
            (class_specifier
                name: (type_identifier) @name) @class
            (struct_specifier
                name: (type_identifier) @name) @struct
        "#).context("Failed to create C++ hpp query")?;
        self.parsers.insert("hpp".to_string(), (cpp_hpp_language, cpp_hpp_parser, cpp_hpp_query));
        
        // Java
        let java_language = tree_sitter_java::language();
        let mut java_parser = Parser::new();
        java_parser.set_language(java_language).context("Failed to set Java language")?;
        let java_query = Query::new(java_language, r#"
            (method_declaration
                name: (identifier) @name) @method
            (class_declaration
                name: (identifier) @name) @class
            (interface_declaration
                name: (identifier) @name) @interface
        "#).context("Failed to create Java query")?;
        
        self.parsers.insert("java".to_string(), (java_language, java_parser, java_query));
        
        // C#
        let csharp_language = tree_sitter_c_sharp::language();
        let mut csharp_parser = Parser::new();
        csharp_parser.set_language(csharp_language).context("Failed to set C# language")?;
        let csharp_query = Query::new(csharp_language, r#"
            (method_declaration
                name: (identifier) @name) @method
            (class_declaration
                name: (identifier) @name) @class
            (interface_declaration
                name: (identifier) @name) @interface
            (struct_declaration
                name: (identifier) @name) @struct
        "#).context("Failed to create C# query")?;
        
        self.parsers.insert("cs".to_string(), (csharp_language, csharp_parser, csharp_query));
        
        Ok(())
    }
    
    
    pub fn chunk_code_for_indexing(&mut self, file_path: &Path, content: &str) -> Result<Vec<CodeChunk>> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if let Some((_language, parser, query)) = self.parsers.get_mut(extension) {
            let tree = parser.parse(content, None).context("Failed to parse file")?;
            let root_node = tree.root_node();
            
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(query, root_node, content.as_bytes());
            
            let mut chunks = Vec::new();
            
            // First pass: collect all methods, functions, classes
            for match_ in matches {
                let mut chunk_node = None;
                let mut name = String::new();
                let mut chunk_type = ChunkType::Other;
                
                for capture in match_.captures {
                    let capture_name = &query.capture_names()[capture.index as usize];
                    let node = capture.node;
                    
                    match capture_name.as_str() {
                        "name" => {
                            name = node.utf8_text(content.as_bytes())
                                .unwrap_or("")
                                .to_string();
                        }
                        "function" => {
                            chunk_node = Some(node);
                            chunk_type = ChunkType::Function;
                        }
                        "method" => {
                            chunk_node = Some(node);
                            chunk_type = ChunkType::Method;
                        }
                        "class" => {
                            chunk_node = Some(node);
                            chunk_type = ChunkType::Class;
                        }
                        "struct" => {
                            chunk_node = Some(node);
                            chunk_type = ChunkType::Struct;
                        }
                        "interface" => {
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
                    
                    let chunk_content = if matches!(chunk_type, ChunkType::Class | ChunkType::Struct | ChunkType::Interface) {
                        // For classes, extract only declaration + fields, exclude methods
                        Self::extract_class_declaration_with_fields_static(content, start_byte, end_byte)?
                    } else {
                        // For methods/functions, include full content
                        content
                            .get(start_byte..end_byte)
                            .unwrap_or("")
                            .to_string()
                    };
                    
                    // Skip if content is mostly imports
                    if !Self::is_mostly_imports_static(&chunk_content) {
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
        } else {
            // For unsupported file types, return the entire file as one chunk (excluding imports)
            let filtered_content = Self::filter_imports_static(content);
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
    
    fn extract_class_declaration_with_fields_static(content: &str, start_byte: usize, end_byte: usize) -> Result<String> {
        let class_content = content.get(start_byte..end_byte).unwrap_or("");
        let lines: Vec<&str> = class_content.lines().collect();
        
        let mut result_lines = Vec::new();
        let mut brace_count = 0;
        let mut in_method = false;
        let mut method_brace_count = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                if !in_method {
                    result_lines.push(line.to_string());
                }
                continue;
            }
            
            // Count braces to track nesting
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            
            // Detect method start (simplified heuristic)
            if (trimmed.contains("public ") || trimmed.contains("private ") || trimmed.contains("protected ") || 
                trimmed.contains("fn ") || trimmed.contains("func ") || trimmed.contains("def ")) &&
               (trimmed.contains('(') && trimmed.contains(')')) {
                in_method = true;
                method_brace_count = brace_count;
                continue; // Skip method definitions
            }
            
            // If we're in a method, skip until we're out
            if in_method {
                if brace_count < method_brace_count {
                    in_method = false;
                }
                continue;
            }
            
            // Include class declaration, annotations, fields, but not methods
            if !in_method {
                result_lines.push(line.to_string());
            }
        }
        
        Ok(result_lines.join("\n"))
    }
    
    fn is_mostly_imports_static(content: &str) -> bool {
        let lines: Vec<&str> = content.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("/*"))
            .collect();
        
        if lines.is_empty() {
            return true;
        }
        
        let import_lines = lines.iter()
            .filter(|line| {
                line.starts_with("import ") || line.starts_with("use ") || 
                line.starts_with("from ") || line.starts_with("#include") ||
                line.starts_with("using ")
            })
            .count();
        
        // If more than 80% of lines are imports, consider it mostly imports
        (import_lines as f64 / lines.len() as f64) > 0.8
    }
    
    fn filter_imports_static(content: &str) -> String {
        content.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("import ") && !trimmed.starts_with("use ") && 
                !trimmed.starts_with("from ") && !trimmed.starts_with("#include") &&
                !trimmed.starts_with("using ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn split_method_content(content: &str, chunk_type: &ChunkType) -> (String, String) {
        match chunk_type {
            ChunkType::Function | ChunkType::Method => {
                Self::split_function_declaration_and_body(content)
            }
            ChunkType::Class | ChunkType::Struct | ChunkType::Interface => {
                // For classes/structs, declaration is the header, body is the rest
                Self::split_class_declaration_and_body(content)
            }
            _ => {
                // For other chunk types, treat whole content as declaration
                (content.to_string(), String::new())
            }
        }
    }

    fn split_function_declaration_and_body(content: &str) -> (String, String) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return (String::new(), String::new());
        }

        let mut declaration_lines = Vec::new();
        let mut body_lines = Vec::new();
        let mut found_opening_brace = false;
        for line in lines {

            if !found_opening_brace {
                declaration_lines.push(line);
                
                // Count braces to find the opening brace of the function body
                let open_braces = line.matches('{').count() as i32;
                let close_braces = line.matches('}').count() as i32;
                // Track brace balance (currently not used but may be needed for complex parsing)
                let _brace_count = open_braces - close_braces;

                if open_braces > 0 {
                    found_opening_brace = true;
                    // If the line has content after the opening brace, start body from this line
                    let brace_pos = line.find('{').unwrap();
                    if brace_pos + 1 < line.len() && line[brace_pos + 1..].trim().len() > 0 {
                        body_lines.push(line);
                    }
                }
            } else {
                body_lines.push(line);
            }
        }

        // If no opening brace found, treat entire content as declaration
        if !found_opening_brace {
            return (content.to_string(), String::new());
        }

        let declaration = declaration_lines.join("\n");
        let body = if body_lines.is_empty() {
            String::new()
        } else {
            body_lines.join("\n")
        };

        (declaration, body)
    }

    fn split_class_declaration_and_body(content: &str) -> (String, String) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return (String::new(), String::new());
        }

        let mut declaration_lines = Vec::new();
        let mut body_lines = Vec::new();
        let mut in_class_body = false;

        for line in lines {
            let trimmed = line.trim();
            
            if !in_class_body {
                declaration_lines.push(line);
                
                // Check if this line ends the class declaration (contains opening brace)
                if trimmed.contains('{') {
                    in_class_body = true;
                }
            } else {
                body_lines.push(line);
            }
        }

        let declaration = declaration_lines.join("\n");
        let body = if body_lines.is_empty() {
            String::new()
        } else {
            body_lines.join("\n")
        };

        (declaration, body)
    }
}