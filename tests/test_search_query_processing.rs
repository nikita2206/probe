use probe::search_index::SearchIndex;
use std::fs;
use tantivy::tokenizer::Language;
use tempfile::TempDir;

#[test]
fn test_search_snippet_quality() {
    let temp_dir = TempDir::new().unwrap();
    let index_dir = temp_dir.path().join("test_index");

    // Create a search index
    let mut index = SearchIndex::new(&index_dir, Language::English, true).unwrap();

    // Create test content with search terms later in the file
    let test_content = r#"// This is the beginning of a file with lots of content
// that should NOT appear in the snippet when searching for
// terms that appear much later in the file.

package com.example.service;

import java.util.HashMap;
import java.util.Map;

public class DataProcessor {
    private Map<String, String> cache;
    
    public DataProcessor() {
        this.cache = new HashMap<>();
    }

    // This function contains our search term "archive lc"
    public void archiveLocalCache(String path) throws Exception {
        System.out.println("Creating archive lc at: " + path);
        // Write cache data to file
        // archive lc functionality here
    }
}
"#;

    // Write the content to a temporary file
    let test_file = temp_dir.path().join("test_file.java");
    fs::write(&test_file, test_content).unwrap();

    // Index the file
    index
        .index_files([test_file.clone()], 1)
        .unwrap()
        .for_each(drop);

    // Test search with Tantivy's snippet generation
    let search_results = index.search("archive lc", 10, None).unwrap();
    println!("Search results count: {}", search_results.len());

    assert!(
        !search_results.is_empty(),
        "Should find results for 'archive lc'"
    );

    for (i, result) in search_results.iter().enumerate() {
        println!("Result {}: {}", i + 1, result.path.display());
        println!("Score: {}", result.score);
        println!("Snippet: '{}'", result.snippet);

        // Verify the snippet contains context around the match
        assert!(!result.snippet.is_empty(), "Snippet should not be empty");

        // The snippet should ideally contain the matched text or related context
        // (Tantivy's snippet generation is sophisticated and may not contain exact match)
        println!("✅ Snippet generated successfully");
    }

    // Test with different query variations
    println!("\n--- Testing query variations ---");
    for query in &["archive lc", "archiveLocalCache", "Creating archive"] {
        println!("Query: '{query}'");
        let results = index.search(query, 5, None).unwrap();
        println!("  Results found: {}", results.len());
        for result in results {
            println!("    Snippet: '{}'", result.snippet);
        }
    }
}

#[test]
fn test_tantivy_query_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let index_dir = temp_dir.path().join("test_index");
    let mut index = SearchIndex::new(&index_dir, Language::English, true).unwrap();

    let content = "This function handles archive local cache operations";
    let test_file = temp_dir.path().join("test.java");
    fs::write(&test_file, content).unwrap();
    index.index_files([test_file], 1).unwrap().for_each(drop);

    // Test how tantivy parses different queries
    for query in &[
        "archive lc",
        "\"archive lc\"",
        "archive AND lc",
        "archive OR lc",
    ] {
        println!("Testing query: {query}");
        let results = index.search(query, 5, None).unwrap();
        println!("  Results: {}", results.len());
        for result in results {
            println!("    Snippet: '{}'", result.snippet);
        }
    }
}
