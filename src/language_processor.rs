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
    
    /// Splits method/function content into declaration and body
    fn split_method_content(&self, content: &str, chunk_type: &ChunkType) -> (String, String);
}

/// Utility functions shared across language processors
pub mod utils {
    use super::*;

    pub fn extract_class_declaration_with_fields(
        content: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> Result<String> {
        let class_content = content.get(start_byte..end_byte).unwrap_or("");
        let lines: Vec<&str> = class_content.lines().collect();

        let mut result_lines = Vec::new();
        let mut brace_count = 0;
        let mut in_method = false;
        let mut method_brace_count = 0;

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("*")
            {
                if !in_method {
                    result_lines.push(line.to_string());
                }
                continue;
            }

            // Count braces to track nesting
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;

            // Detect method start (simplified heuristic)
            if (trimmed.contains("public ")
                || trimmed.contains("private ")
                || trimmed.contains("protected ")
                || trimmed.contains("fn ")
                || trimmed.contains("func ")
                || trimmed.contains("def "))
                && (trimmed.contains('(') && trimmed.contains(')'))
            {
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

    pub fn is_mostly_imports(content: &str) -> bool {
        let lines: Vec<&str> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("/*"))
            .collect();

        if lines.is_empty() {
            return true;
        }

        let import_lines = lines
            .iter()
            .filter(|line| {
                line.starts_with("import ")
                    || line.starts_with("use ")
                    || line.starts_with("from ")
                    || line.starts_with("#include")
                    || line.starts_with("using ")
            })
            .count();

        // If more than 80% of lines are imports, consider it mostly imports
        (import_lines as f64 / lines.len() as f64) > 0.8
    }

    pub fn filter_imports(content: &str) -> String {
        content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("import ")
                    && !trimmed.starts_with("use ")
                    && !trimmed.starts_with("from ")
                    && !trimmed.starts_with("#include")
                    && !trimmed.starts_with("using ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn split_function_declaration_and_body(content: &str) -> (String, String) {
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
                    if brace_pos + 1 < line.len() && !line[brace_pos + 1..].trim().is_empty() {
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

    pub fn split_class_declaration_and_body(content: &str) -> (String, String) {
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