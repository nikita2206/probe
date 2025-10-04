use crate::language_processor::{ChunkType, LanguageProcessor};
use crate::languages::java::JavaProcessor;
use crate::tests::indent_string;
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
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract 1 class chunk + 2 method chunks = 3 chunks
    assert_eq!(
        chunks.len(),
        3,
        "Should extract exactly 3 chunks (1 class + 2 methods) but got {}. Extracted chunks: {:?}",
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

    // First chunk should be the class
    let class_chunk = &chunks[0];
    assert_eq!(class_chunk.chunk_type, ChunkType::Class);
    assert_eq!(class_chunk.name, "FooBar");

    // Second chunk should be for someMethod
    let first_chunk = &chunks[1];
    assert_eq!(first_chunk.chunk_type, ChunkType::Method);
    assert_eq!(first_chunk.name, "someMethod");

    // Third chunk should be for doSomething method
    let second_chunk = &chunks[2];
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

    // Should extract 1 class chunk + 2 method chunks = 3 chunks
    assert_eq!(
        chunks.len(),
        3,
        "Should extract exactly 3 chunks (1 class + 2 methods) but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk should be the class
    let class_chunk = &chunks[0];
    assert_eq!(class_chunk.chunk_type, ChunkType::Class);
    assert_eq!(class_chunk.name, "MultiLineClass");

    // Second chunk should be for veryLongMethod
    let first_chunk = &chunks[1];
    assert_eq!(first_chunk.chunk_type, ChunkType::Method);
    assert_eq!(first_chunk.name, "veryLongMethod");

    // Third chunk should be for anotherMethod
    let second_chunk = &chunks[2];
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

    let expected_first_content = indent_string(
        indoc! {r#"
                // method body
                System.out.println(firstParameter);
            }"#},
        4,
    );

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

    let expected_second_content = indent_string(
        indoc! {r#"
                return a + b;
            }"#},
        4,
    );

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
fn test_java_class_declaration_chunking() {
    let java_code = indoc! {r#"
        package com.example.model;

        /**
         * User entity class
         * Represents a user in the system
         */
        public class User {
            /** User's unique identifier */
            private String id;
            
            private String username;
            
            /** User's email address */
            private String email = "default@example.com";
            
            public User(String id, String username, String email) {
                this.id = id;
                this.username = username;
                this.email = email;
            }
            
            public String getId() { return id; }
            public void setId(String id) { this.id = id; }
        }
    "#};

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract 1 class chunk + 3 method chunks = 4 total
    assert_eq!(
        chunks.len(),
        4,
        "Should extract exactly 4 chunks (1 class + 3 methods) but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk should be the Class chunk
    let class_chunk = &chunks[0];
    assert_eq!(class_chunk.chunk_type, ChunkType::Class);
    assert_eq!(class_chunk.name, "User");

    // Class declaration should include javadoc and class header
    assert!(
        class_chunk.declaration.contains("User entity class"),
        "Class declaration should include class javadoc"
    );
    assert!(
        class_chunk.declaration.contains("public class User {"),
        "Class declaration should include class header"
    );

    // Class content should include fields with javadocs and initializers
    assert!(
        class_chunk.content.contains("User's unique identifier"),
        "Class content should include field javadoc"
    );
    assert!(
        class_chunk.content.contains("private String id;"),
        "Class content should include field declarations"
    );
    assert!(
        class_chunk.content.contains("User's email address"),
        "Class content should include field javadoc for email"
    );
    // The initializer should be included
    assert!(
        class_chunk.content.contains("default@example.com"),
        "Class content should include field initializers"
    );

    // Remaining chunks should be methods
    assert_eq!(chunks[1].chunk_type, ChunkType::Method);
    assert_eq!(chunks[1].name, "User"); // constructor
    assert_eq!(chunks[2].chunk_type, ChunkType::Method);
    assert_eq!(chunks[2].name, "getId");
    assert_eq!(chunks[3].chunk_type, ChunkType::Method);
    assert_eq!(chunks[3].name, "setId");
}

#[test]
fn test_java_nested_class_declaration_chunking() {
    let java_code = indoc! {r#"
        package com.example;

        /**
         * Outer class
         */
        public class OuterClass {
            private String outerField;
            
            /**
             * Inner class
             */
            public static class InnerClass {
                /** Inner field */
                private int innerField;
                
                public void innerMethod() {
                    System.out.println("Inner");
                }
            }
            
            public void outerMethod() {
                System.out.println("Outer");
            }
        }
    "#};

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract: 1 outer class + 1 inner class + 2 methods = 4 total
    assert_eq!(
        chunks.len(),
        4,
        "Should extract exactly 4 chunks but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk: OuterClass
    let outer_class_chunk = &chunks[0];
    assert_eq!(outer_class_chunk.chunk_type, ChunkType::Class);
    assert_eq!(outer_class_chunk.name, "OuterClass");
    assert!(
        outer_class_chunk
            .content
            .contains("private String outerField"),
        "Outer class content should include its field"
    );

    // Second chunk: InnerClass
    let inner_class_chunk = &chunks[1];
    assert_eq!(inner_class_chunk.chunk_type, ChunkType::Class);
    assert_eq!(inner_class_chunk.name, "InnerClass");

    // Inner class declaration should include compact outer class context
    assert!(
        inner_class_chunk
            .declaration
            .contains("public class OuterClass {"),
        "Inner class declaration should include compact outer class"
    );
    assert!(
        inner_class_chunk.declaration.contains("Inner class"),
        "Inner class declaration should include its own javadoc"
    );

    // Inner class content should include its own fields
    assert!(
        inner_class_chunk.content.contains("Inner field"),
        "Inner class content should include field javadoc"
    );
    assert!(
        inner_class_chunk.content.contains("private int innerField"),
        "Inner class content should include its field"
    );

    // Third and fourth chunks: methods
    let inner_method = &chunks[2];
    assert_eq!(inner_method.chunk_type, ChunkType::Method);
    assert_eq!(inner_method.name, "innerMethod");

    let outer_method = &chunks[3];
    assert_eq!(outer_method.chunk_type, ChunkType::Method);
    assert_eq!(outer_method.name, "outerMethod");
}

#[test]
fn test_java_interface_declaration_chunking() {
    let java_code = indoc! {r#"
        package com.example.service;

        /**
         * Service interface for user operations
         */
        public interface UserService {
            
            /**
             * Retrieves a user by their ID
             * @param userId the unique identifier for the user
             * @return the user object if found, null otherwise
             */
            User getUserById(String userId);
            
            /**
             * Creates a new user account
             */
            User createUser(String username, String email);
        }
    "#};

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract: 1 interface + 2 methods = 3 total
    assert_eq!(
        chunks.len(),
        3,
        "Should extract exactly 3 chunks but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk should be the Interface chunk (stored as Class type)
    let interface_chunk = &chunks[0];
    assert_eq!(interface_chunk.chunk_type, ChunkType::Class);
    assert_eq!(interface_chunk.name, "UserService");

    assert!(
        interface_chunk
            .declaration
            .contains("Service interface for user operations"),
        "Interface declaration should include interface javadoc"
    );
    assert!(
        interface_chunk
            .declaration
            .contains("public interface UserService {"),
        "Interface declaration should include interface header"
    );

    // Remaining chunks should be methods
    assert_eq!(chunks[1].chunk_type, ChunkType::Method);
    assert_eq!(chunks[1].name, "getUserById");
    assert_eq!(chunks[2].chunk_type, ChunkType::Method);
    assert_eq!(chunks[2].name, "createUser");
}

#[test]
fn test_java_record_declaration_chunking() {
    let java_code = indoc! {r#"
        package com.example.model;

        import java.time.LocalDate;

        /**
         * A record representing a person with basic information
         */
        public record Person(String name, int age, String email) {
            
            /**
             * Creates a person with default values
             */
            public static Person withDefaults(String name) {
                return new Person(name, 0, "unknown@example.com");
            }
            
            /**
             * Instance method to check if person is an adult
             */
            public boolean isAdult() {
                return age >= 18;
            }
        }
    "#};

    let mut processor = JavaProcessor::new().expect("Failed to create JavaProcessor");
    let chunks = processor
        .chunk_code(java_code)
        .expect("Failed to chunk Java code");

    // Should extract: 1 record + 2 methods = 3 chunks
    assert_eq!(
        chunks.len(),
        3,
        "Should extract exactly 3 chunks (1 record + 2 methods) but got {}. Extracted chunks: {:?}",
        chunks.len(),
        chunks
            .iter()
            .map(|c| format!("{:?} - {}", c.chunk_type, c.name))
            .collect::<Vec<_>>()
    );

    // First chunk should be the Record (stored as Class type)
    let record_chunk = &chunks[0];
    assert_eq!(record_chunk.chunk_type, ChunkType::Class);
    assert_eq!(record_chunk.name, "Person");

    // Record declaration should include javadoc and full record header with parameters
    assert!(
        record_chunk
            .declaration
            .contains("A record representing a person"),
        "Record declaration should include record javadoc"
    );
    assert!(
        record_chunk
            .declaration
            .contains("public record Person(String name, int age, String email) {"),
        "Record declaration should include record header with all parameters"
    );

    // Record content should be empty or minimal since records don't typically have fields
    // (the parameters in the record declaration ARE the fields)

    // Remaining chunks should be methods
    assert_eq!(chunks[1].chunk_type, ChunkType::Method);
    assert_eq!(chunks[1].name, "withDefaults");
    assert_eq!(chunks[2].chunk_type, ChunkType::Method);
    assert_eq!(chunks[2].name, "isAdult");
}
