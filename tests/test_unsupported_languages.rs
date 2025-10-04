use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_unsupported_language_python_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create a Python file (unsupported language)
    let python_file = temp_dir.path().join("calculator.py");
    let python_content = r#"# Python Calculator Module
# This module provides basic calculation functions

def add_numbers(x, y):
    """Add two numbers together"""
    result = x + y
    return result

def subtract_numbers(x, y):
    """Subtract y from x"""
    result = x - y
    return result

def multiply_numbers(x, y):
    """Multiply two numbers"""
    result = x * y
    return result

def divide_numbers(x, y):
    """Divide x by y"""
    if y == 0:
        raise ValueError("Cannot divide by zero")
    result = x / y
    return result

def main():
    a = 10
    b = 5
    print(f"Addition: {add_numbers(a, b)}")
    print(f"Subtraction: {subtract_numbers(a, b)}")
    print(f"Multiplication: {multiply_numbers(a, b)}")
    print(f"Division: {divide_numbers(a, b)}")

if __name__ == "__main__":
    main()
"#;
    fs::write(&python_file, python_content).unwrap();

    // Build index
    Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    // Test search for a function that appears in the middle of the file
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("multiply_numbers")
        .arg("--no-rerank")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);

    // Should find the file
    assert!(
        output_str.contains("calculator.py"),
        "Should find calculator.py"
    );

    // Should show context around the match (with default -C 3)
    assert!(
        output_str.contains("multiply_numbers"),
        "Should contain the matched term"
    );

    // Should include some context lines
    assert!(
        output_str.contains("def") || output_str.contains("result"),
        "Should show context lines around the match"
    );
}

#[test]
fn test_unsupported_language_javascript_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create a JavaScript file (unsupported language)
    let js_file = temp_dir.path().join("utils.js");
    let js_content = r#"// Utility functions for string manipulation
// Author: Test User

function capitalizeString(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

function reverseString(str) {
    return str.split('').reverse().join('');
}

function truncateString(str, maxLength) {
    if (str.length <= maxLength) {
        return str;
    }
    return str.substring(0, maxLength) + '...';
}

// Export functions
module.exports = {
    capitalizeString,
    reverseString,
    truncateString
};
"#;
    fs::write(&js_file, js_content).unwrap();

    // Build index
    Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    // Test search with custom context lines
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("truncateString")
        .arg("--no-rerank")
        .arg("-C")
        .arg("2")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);
    let stderr_str = String::from_utf8_lossy(&output.get_output().stderr);

    // Should find the file in stderr
    assert!(
        stderr_str.contains("truncateString"),
        "Should mention search term in stderr"
    );
    assert!(
        stderr_str.contains("Found"),
        "Should show results found message"
    );

    // Should show the file path
    assert!(
        output_str.contains("utils.js"),
        "Should show utils.js in results"
    );
}

#[test]
fn test_file_size_limit() {
    let temp_dir = TempDir::new().unwrap();

    // Create a large file (> 512KB)
    let large_file = temp_dir.path().join("large.txt");
    let large_content = "x".repeat(600 * 1024); // 600KB
    fs::write(&large_file, large_content).unwrap();

    // Create a normal file
    let normal_file = temp_dir.path().join("normal.txt");
    fs::write(&normal_file, "This is a searchable normal file").unwrap();

    // Build index
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);

    // The large file should be skipped silently (no error message)
    // Only the normal file should be indexed
    assert!(output_str.contains("1 files indexed") || output_str.contains("1 file indexed"));
}

#[test]
fn test_long_line_limit() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with a very long line (> 8096 bytes)
    let long_line_file = temp_dir.path().join("minified.js");
    let long_line = "var x = ".to_string() + &"a".repeat(9000) + ";";
    fs::write(&long_line_file, long_line).unwrap();

    // Create a normal file
    let normal_file = temp_dir.path().join("normal.js");
    fs::write(&normal_file, "var y = 123;").unwrap();

    // Build index
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);

    // The long line file should be skipped silently
    // Only the normal file should be indexed
    assert!(output_str.contains("1 files indexed") || output_str.contains("1 file indexed"));
}

#[test]
fn test_multiple_unsupported_languages() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in various unsupported languages
    let python_file = temp_dir.path().join("script.py");
    fs::write(&python_file, "def search_function():\n    pass").unwrap();

    let ruby_file = temp_dir.path().join("script.rb");
    fs::write(&ruby_file, "def search_function\n  puts 'searching'\nend").unwrap();

    let go_file = temp_dir.path().join("main.go");
    fs::write(&go_file, "func searchFunction() {\n  // search logic\n}").unwrap();

    // Build index
    Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    // Search for "search"
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("search")
        .arg("--no-rerank")
        .arg("-n")
        .arg("10")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);

    // Should find results in all three files
    assert!(output_str.contains("script.py"), "Should find Python file");
    assert!(output_str.contains("script.rb"), "Should find Ruby file");
    assert!(output_str.contains("main.go"), "Should find Go file");
}

#[test]
fn test_context_lines_parameter() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with multiple distinct sections
    let test_file = temp_dir.path().join("test.txt");
    let content = r#"Line 1
Line 2
Line 3
Line 4
Line 5
Line 6: This line contains the TARGET word
Line 7
Line 8
Line 9
Line 10
"#;
    fs::write(&test_file, content).unwrap();

    // Build index
    Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("rebuild")
        .assert()
        .success();

    // Search with -C 1 (1 line of context)
    let output = Command::cargo_bin("probe")
        .unwrap()
        .current_dir(temp_dir.path())
        .arg("TARGET")
        .arg("--no-rerank")
        .arg("-C")
        .arg("1")
        .assert()
        .success();

    let output_str = String::from_utf8_lossy(&output.get_output().stdout);
    let stderr_str = String::from_utf8_lossy(&output.get_output().stderr);

    eprintln!("STDOUT: {}", output_str);
    eprintln!("STDERR: {}", stderr_str);

    // Search should find the file and term
    assert!(
        stderr_str.contains("TARGET") || output_str.contains("TARGET"),
        "Should contain the matched term in output"
    );
}
