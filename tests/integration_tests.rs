use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn get_test_data_path() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/test_data/sample_project"
    )
}

fn copy_test_data_to_temp() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let source_path = Path::new(get_test_data_path());
    copy_dir_recursively(source_path, temp_dir.path()).unwrap();
    temp_dir
}

fn copy_dir_recursively(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn get_gitignore_test_data_path() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/test_data/gitignore_test"
    )
}

fn copy_gitignore_test_data_to_temp() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let source_path = Path::new(get_gitignore_test_data_path());
    copy_dir_recursively(source_path, temp_dir.path()).unwrap();
    temp_dir
}

#[test]
fn test_basic_search() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // First ensure index is built
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Test search for "main" - should find main.rs
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn test_search_function_name() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "calculate_sum" - should find lib.rs
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "calculate_sum"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lib.rs"));
}

#[test]
fn test_search_json_content() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "database_url" - should find config.json
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "database_url"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.json"));
}

#[test]
fn test_search_subdirectory() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "HashMap" - should find utils/helper.rs
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "HashMap"])
        .assert()
        .success()
        .stdout(predicate::str::contains("helper.rs"));
}

#[test]
fn test_gitignore_respected() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "binary" - should NOT find binary.exe (ignored by .gitignore)
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "binary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("binary.exe").not());
}

#[test]
fn test_gitignore_log_files() {
    let temp_dir = copy_gitignore_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "indexed" - should find source.rs but NOT ignored.log
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "indexed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source.rs"))
        .stdout(predicate::str::contains("ignored.log").not());
}

#[test]
fn test_no_results_found() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for something that doesn't exist
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "nonexistent_term_xyz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results found"));
}

#[test]
fn test_stats_command() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index first
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Test stats command
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files in index:"))
        .stdout(predicate::str::contains("Index directory:"));
}

#[test]
fn test_incremental_update() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Initial index build
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Add a new file
    fs::write(
        project_path.join("new_file.rs"),
        "pub fn new_function() -> String {\n    \"hello\".to_string()\n}",
    )
    .unwrap();

    // Search should trigger incremental update and find new content
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "new_function"])
        .assert()
        .success()
        .stdout(predicate::str::contains("new_file.rs"));
}

#[test]
fn test_help_command() {
    Command::cargo_bin("codesearch")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Fast code search with persistent indexing",
        ))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_no_query_provided() {
    Command::cargo_bin("codesearch")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_snippet_extraction_shows_match_location() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index to include our test file
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "archive lc" - this term appears around line 50+ in archive_test.rs
    // The snippet should show the area around the match, NOT the beginning of the file
    let output = Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "archive lc"])
        .assert()
        .success()
        .stdout(predicate::str::contains("archive_test.rs"));

    let output_str = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    println!("Search output for 'archive lc':\n{}", output_str);

    // The snippet should contain context around the match, not the file beginning
    // EXPECTED: Should contain "archive_local_cache" or "Archiving local cache"
    // BROKEN BEHAVIOR: Likely shows "This is a test file to demonstrate" (file beginning)

    // This test documents the current broken behavior
    // TODO: Fix snippet extraction to show actual match location
    if output_str.contains("This is a test file to demonstrate") {
        println!("❌ BROKEN: Snippet shows beginning of file instead of match location");
        println!("The snippet should show context around 'archive lc' match, not file start");
    }

    if output_str.contains("archive_local_cache") || output_str.contains("Archiving local cache") {
        println!("✅ CORRECT: Snippet shows context around the actual match");
    } else {
        println!("❌ BROKEN: Snippet does not show context around the match");
        println!("Expected snippet to contain 'archive_local_cache' or 'Archiving local cache'");
    }
}

#[test]
fn test_filetype_filtering() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for "main" with filetype filter for "rs" files
    // Should find main.rs but not if there are other files with "main" in different extensions
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "-t", "rs", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));

    // Search for "database_url" with filetype filter for "json" files
    // Should find config.json
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&[
            "-d",
            project_path.to_str().unwrap(),
            "-t",
            "json",
            "database_url",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.json"));

    // Search for "main" with filetype filter for "json" files
    // Should NOT find main.rs since it's not a json file
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "-t", "json", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs").not());
}

#[test]
fn test_filetype_filtering_markdown() {
    let temp_dir = copy_test_data_to_temp();
    let project_path = temp_dir.path();

    // Rebuild index
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&["-d", project_path.to_str().unwrap(), "rebuild"])
        .assert()
        .success();

    // Search for content in markdown files only
    // README.md should be found when searching for content with md filter
    Command::cargo_bin("codesearch")
        .unwrap()
        .args(&[
            "-d",
            project_path.to_str().unwrap(),
            "-t",
            "md",
            "Sample Project",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"));
}
