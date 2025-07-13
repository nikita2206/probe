use crate::languages::java::JavaProcessor;
use crate::language_processor::{LanguageProcessor, ChunkType};
use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn test_java_class_method_chunking() {
    let java_code = r#"package com.bla.foo;

import static com.bla.bar.Some.blablaCode;

/**
 * Long JavaDoc description
 */
class FooBar implements SomeInterface {
    void someMethod() {
        blablaCode();
    }

    String doSomething() {
        this.someMethod();
        return "text";
    }
}
"#;

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor.chunk_code(java_code).expect("Failed to chunk Java code");

    // Should extract exactly 2 chunks
    assert_eq!(
        chunks.len(), 
        2, 
        "Should extract exactly 2 chunks but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks.iter().map(|c| format!("{:?} - {}", c.chunk_type, c.name)).collect::<Vec<_>>()
    );

    // Each chunk should contain:
    // 1. The class declaration with JavaDoc
    // 2. A placeholder for other methods (// ...)
    // 3. The specific method implementation

    // First chunk should be for someMethod
    let first_chunk = &chunks[0];
    assert_eq!(first_chunk.chunk_type, ChunkType::Method);
    assert_eq!(first_chunk.name, "someMethod");

    // Second chunk should be for doSomething method
    let second_chunk = &chunks[1];
    assert_eq!(second_chunk.chunk_type, ChunkType::Method);
    assert_eq!(second_chunk.name, "doSomething");

    // Expected content for first chunk: class declaration + someMethod
    let expected_first_chunk = indoc! {r#"
        /**
         * Long JavaDoc description
         */
        class FooBar implements SomeInterface {
            // ...
            void someMethod() {
                blablaCode();
            }
    "#};

    // Expected content for second chunk: class declaration + doSomething
    let expected_second_chunk = indoc! {r#"
        /**
         * Long JavaDoc description
         */
        class FooBar implements SomeInterface {
            // ...
            String doSomething() {
                this.someMethod();
                return "text";
            }
    "#};

    assert_eq!(
        first_chunk.content.trim(),
        expected_first_chunk.trim(),
        "First chunk content should match expected structure"
    );

    assert_eq!(
        second_chunk.content.trim(),
        expected_second_chunk.trim(),
        "Second chunk content should match expected structure"
    );
} 