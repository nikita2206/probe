use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn copy_test_data(temp_dir: &Path, source: &str) -> std::io::Result<()> {
    let source_path = Path::new("tests/test_data").join(source);
    let dest_path = temp_dir.join(source);

    // Create parent directories if they don't exist
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Copy the entire directory recursively
    copy_dir_all(&source_path, &dest_path)?;
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[test]
fn test_java_interface_method_declarations_appear_in_search_results() {
    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Copy test data
    copy_test_data(temp_path, "java_interface_test").expect("Failed to copy test data");

    // Run search for method declaration that should appear in results
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("getUserById")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that we found the method
    assert!(
        stdout.contains("getUserById"),
        "Method name should be found in search results"
    );

    // Verify that the method declaration is included in the results
    assert!(
        stdout.contains("User getUserById(String userId)"),
        "Method declaration should appear in search results"
    );

    // Additional checks to ensure we get the full context
    assert!(
        stdout.contains("UserService.java"),
        "File name should be mentioned"
    );

    // The method should be identified as a Method
    assert!(
        stdout.contains("Method getUserById"),
        "Should identify this as a method"
    );
}

#[test]
fn test_java_interface_multiple_methods_with_same_pattern() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    copy_test_data(temp_path, "java_interface_test").expect("Failed to copy test data");

    // Search for a specific interface method
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("createUser")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should find the createUser method
    assert!(
        stdout.contains("createUser"),
        "Should find createUser method"
    );

    // Verify method declarations are visible in results
    assert!(
        stdout.contains("User createUser(String username, String email)"),
        "createUser method declaration should appear in results"
    );
}

#[test]
fn test_java_class_methods_work_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    copy_test_data(temp_path, "java_interface_test").expect("Failed to copy test data");

    // Search for class methods (these should work fine)
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("getId")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Class methods should show their implementations
    assert!(stdout.contains("getId"), "Should find getId method");
    assert!(
        stdout.contains("public String getId()"),
        "Should show method implementation"
    );
    assert!(stdout.contains("return id;"), "Should show method body");
}
