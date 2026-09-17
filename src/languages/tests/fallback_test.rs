use crate::code_chunker::CodeChunker;
use crate::language_processor::{ChunkType, LanguageProcessor};
use crate::languages::fallback::FallbackProcessor;
use pretty_assertions::assert_eq;
use std::path::Path;

fn chunk_file(path: &str, content: &str) -> Vec<crate::language_processor::CodeChunk> {
    let mut chunker = CodeChunker::new().expect("Failed to create CodeChunker");
    chunker
        .chunk_code_for_indexing(Path::new(path), content)
        .expect("Failed to chunk")
}

#[test]
fn fallback_indexes_nonempty_content_as_one_file_chunk() {
    let content = "def multiply_numbers(x, y):\n    return x * y\n";
    let mut processor = FallbackProcessor::new();
    let chunks = processor.chunk_code(content).expect("chunk");

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunk_type, ChunkType::Other);
    assert_eq!(chunks[0].name, "file");
    assert_eq!(chunks[0].declaration, "");
    assert_eq!(chunks[0].content, content);
    assert_eq!(chunks[0].start_line, 0);
    assert_eq!(chunks[0].end_line, 1);
}

#[test]
fn fallback_skips_empty_and_whitespace_only_content() {
    let mut processor = FallbackProcessor::new();
    assert!(processor.chunk_code("").unwrap().is_empty());
    assert!(processor.chunk_code("   \n\t\n  ").unwrap().is_empty());
}

#[test]
fn fallback_is_not_registered_by_extension() {
    let processor = FallbackProcessor::new();
    assert!(processor.get_file_extensions().is_empty());
}

#[test]
fn chunker_uses_specialized_processor_for_java() {
    let chunker = CodeChunker::new().unwrap();
    assert!(chunker.has_specialized_processor(Path::new("UserService.java")));
    assert!(!chunker.has_specialized_processor(Path::new("script.py")));
    assert!(!chunker.has_specialized_processor(Path::new("Makefile")));
    assert!(!chunker.has_specialized_processor(Path::new("README.md")));
}

#[test]
fn chunker_does_not_use_fallback_for_java() {
    let java = r#"
class FallbackGuard {
    void specialized() {
        return;
    }
}
"#;
    let chunks = chunk_file("FallbackGuard.java", java);
    assert!(
        chunks
            .iter()
            .any(|c| c.chunk_type == ChunkType::Class && c.name == "FallbackGuard"),
        "Java files must be handled by the specialized processor, got: {:?}",
        chunks
            .iter()
            .map(|c| format!("{:?} {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );
    assert!(
        !chunks
            .iter()
            .any(|c| c.chunk_type == ChunkType::Other && c.name == "file"),
        "Java files must not be indexed as a whole-file fallback chunk"
    );
}

#[test]
fn chunker_falls_back_for_python_javascript_markdown_and_extensionless() {
    let samples = [
        ("calc.py", "def add(x, y):\n    return x + y\n"),
        ("utils.js", "function reverseString(s) { return s; }\n"),
        (
            "notes.md",
            "# Search notes\n\nThe fallback indexer covers markdown.\n",
        ),
        ("Makefile", "build:\n\tcargo build --release\n"),
    ];

    for (path, content) in samples {
        let chunks = chunk_file(path, content);
        assert_eq!(
            chunks.len(),
            1,
            "{path} should produce a single fallback chunk"
        );
        assert_eq!(chunks[0].chunk_type, ChunkType::Other);
        assert_eq!(chunks[0].name, "file");
        assert_eq!(chunks[0].content, content);
    }
}
