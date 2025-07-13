use crate::language_processor::{ChunkType, LanguageProcessor};
use crate::languages::java::JavaProcessor;
use indoc::indoc;
use pretty_assertions::assert_eq;
use crate::tests::indent_string;

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
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract exactly 2 chunks
    assert_eq!(
        chunks.len(),
        2,
        "Should extract exactly 2 chunks but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
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

    // Expected declaration for first chunk: class declaration + someMethod signature
    let expected_first_declaration = indoc! {r#"
        /**
         * Long JavaDoc description
         */
        class FooBar implements SomeInterface {
            void someMethod() {
    "#};

    // Expected content for first chunk: method body only
    let expected_first_content = "        blablaCode();\n    }";

    // Expected declaration for second chunk: class declaration + doSomething signature
    let expected_second_declaration = indoc! {r#"
        /**
         * Long JavaDoc description
         */
        class FooBar implements SomeInterface {
            String doSomething() {
    "#};

    // Expected content for second chunk: method body only
    let expected_second_content = "        this.someMethod();\n        return \"text\";\n    }";

    assert_eq!(
        first_chunk.declaration.trim(),
        expected_first_declaration.trim(),
        "First chunk declaration should match expected structure"
    );

    assert_eq!(
        first_chunk.content.trim(),
        expected_first_content.trim(),
        "First chunk content should match expected structure"
    );

    assert_eq!(
        second_chunk.declaration.trim(),
        expected_second_declaration.trim(),
        "Second chunk declaration should match expected structure"
    );

    assert_eq!(
        second_chunk.content.trim(),
        expected_second_content.trim(),
        "Second chunk content should match expected structure"
    );
}

#[test]
fn test_java_multiline_declarations_chunking() {
    let java_code = indoc! {r#"
        package com.example.multiline;

        /**
         * Multiline JavaDoc
         */
        class MultiLineClass
            implements InterfaceOne,
                       InterfaceTwo,
                       InterfaceThree {
            public void veryLongMethod(
                int firstParameter,
                String secondParameter,
                List<String> thirdParameter,
                Map<String, Integer> fourthParameter
            ) {
                // method body
                System.out.println(firstParameter);
            }

            String anotherMethod(
                String a,
                String b
            ) {
                return a + b;
            }
        }
    "#};

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract exactly 2 chunks
    assert_eq!(
        chunks.len(),
        2,
        "Should extract exactly 2 chunks but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk should be for veryLongMethod
    let first_chunk = &chunks[0];
    assert_eq!(first_chunk.chunk_type, ChunkType::Method);
    assert_eq!(first_chunk.name, "veryLongMethod");

    // Second chunk should be for anotherMethod
    let second_chunk = &chunks[1];
    assert_eq!(second_chunk.chunk_type, ChunkType::Method);
    assert_eq!(second_chunk.name, "anotherMethod");

    // Expected declaration for first chunk: class declaration + veryLongMethod signature
    let expected_first_declaration = indoc! {r#"
        /**
         * Multiline JavaDoc
         */
        class MultiLineClass
            implements InterfaceOne,
                       InterfaceTwo,
                       InterfaceThree {
            public void veryLongMethod(
                int firstParameter,
                String secondParameter,
                List<String> thirdParameter,
                Map<String, Integer> fourthParameter
            ) {
    "#};

    let expected_first_content = indent_string(indoc! {r#"
                // method body
                System.out.println(firstParameter);
            }"#}, 4);

    let expected_second_declaration = indoc! {r#"
        /**
         * Multiline JavaDoc
         */
        class MultiLineClass
            implements InterfaceOne,
                       InterfaceTwo,
                       InterfaceThree {
            String anotherMethod(
                String a,
                String b
            ) {
    "#};

    let expected_second_content = indent_string(indoc! {r#"
                return a + b;
            }"#}, 4);

    assert_eq!(
        first_chunk.declaration.trim(),
        expected_first_declaration.trim(),
        "First chunk declaration should match expected structure"
    );

    assert_eq!(
        first_chunk.content.trim(),
        expected_first_content.trim(),
        "First chunk content should match expected structure"
    );

    assert_eq!(
        second_chunk.declaration.trim(),
        expected_second_declaration.trim(),
        "Second chunk declaration should match expected structure"
    );

    assert_eq!(
        second_chunk.content.trim(),
        expected_second_content.trim(),
        "Second chunk content should match expected structure"
    );
}
