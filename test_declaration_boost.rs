use std::fs;
use std::path::Path;
use tempfile::TempDir;
use codesearch::search_index::SearchIndex;
use tantivy::tokenizer::Language;

fn main() -> anyhow::Result<()> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_project");
    fs::create_dir_all(&test_dir)?;

    // Create a sample JavaScript file with a method that has "search" in both declaration and body
    let js_content = r#"
class DataManager {
    // This method has "search" in the declaration - should get higher score
    async searchUsers(searchTerm) {
        const users = [];
        for (let user of this.users) {
            // This "search" is in the body - should get lower score
            if (user.name.includes(searchTerm)) {
                users.push(user);
            }
        }
        return users;
    }

    // This method has "search" only in the body - should get lower score
    async findData() {
        const data = await this.database.search("SELECT * FROM table");
        return data;
    }
}
"#;

    let js_file = test_dir.join("manager.js");
    fs::write(&js_file, js_content)?;

    // Create search index
    let index_dir = temp_dir.path().join("index");
    let mut search_index = SearchIndex::new(&index_dir, Language::English, false)?;

    // Index the test file
    search_index.index_files(&[js_file])?;

    // Search for "search" - should prioritize declaration matches
    let results = search_index.search("search", 10, None)?;

    println!("Search results for 'search':");
    for (i, result) in results.iter().enumerate() {
        println!("{}. Score: {:.4}", i + 1, result.score);
        println!("   Path: {}", result.path.display());
        if let Some(chunk_name) = &result.chunk_name {
            println!("   Function: {}", chunk_name);
        }
        if let Some(chunk_type) = &result.chunk_type {
            println!("   Type: {}", chunk_type);
        }
        println!("   Snippet: {}", result.snippet);
        println!();
    }

    // The searchUsers method should score higher than findData method
    // because "search" appears in the declaration of searchUsers but only in body of findData
    if results.len() >= 2 {
        let search_users_result = results.iter().find(|r| {
            r.chunk_name.as_ref().map_or(false, |name| name.contains("searchUsers"))
        });
        
        let find_data_result = results.iter().find(|r| {
            r.chunk_name.as_ref().map_or(false, |name| name.contains("findData"))
        });

        if let (Some(search_users), Some(find_data)) = (search_users_result, find_data_result) {
            println!("Score comparison:");
            println!("  searchUsers (declaration match): {:.4}", search_users.score);
            println!("  findData (body match): {:.4}", find_data.score);
            
            if search_users.score > find_data.score {
                println!("✓ SUCCESS: Declaration boost is working! searchUsers scored higher.");
            } else {
                println!("✗ ISSUE: Declaration boost may not be working. findData scored higher or equal.");
            }
        }
    }

    Ok(())
}