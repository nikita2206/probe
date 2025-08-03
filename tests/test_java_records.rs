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
fn test_java_record_static_methods_appear_in_search_results() {
    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    // Copy test data
    copy_test_data(temp_path, "java_records_test").expect("Failed to copy test data");

    // Run search for static method that should appear in results
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("withDefaults")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that we found the static method
    assert!(
        stdout.contains("withDefaults"),
        "Static method name should be found in search results"
    );

    // Verify that the method declaration is included in the results
    assert!(
        stdout.contains("public static Person withDefaults(String name)"),
        "Static method declaration should appear in search results"
    );

    // Additional checks to ensure we get the full context
    assert!(
        stdout.contains("Person.java"),
        "File name should be mentioned"
    );

    // The method should be found in search results
    assert!(
        stdout.contains("withDefaults"),
        "Should find the static method in search results"
    );
}

#[test]
fn test_java_record_multiple_static_methods() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    copy_test_data(temp_path, "java_records_test").expect("Failed to copy test data");

    // Search for a specific static method in Order record
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("createEmpty")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should find the createEmpty static method
    assert!(
        stdout.contains("createEmpty"),
        "Should find createEmpty static method"
    );

    // Verify method declarations are visible in results
    assert!(
        stdout.contains("public static Order createEmpty()"),
        "createEmpty method declaration should appear in results"
    );

    // Should find it in the Order.java file
    assert!(
        stdout.contains("Order.java"),
        "Should find the method in Order.java file"
    );
}

#[test]
fn test_java_record_instance_methods_work_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    copy_test_data(temp_path, "java_records_test").expect("Failed to copy test data");

    // Search for instance methods (these should work fine too)
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("isAdult")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Instance methods should show their implementations
    assert!(stdout.contains("isAdult"), "Should find isAdult method");
    assert!(
        stdout.contains("public boolean isAdult()"),
        "Should show instance method declaration"
    );
    assert!(
        stdout.contains("return age >= 18;"),
        "Should show method body"
    );
}

#[test]
fn test_java_record_static_method_with_parameters() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_path = temp_dir.path();

    copy_test_data(temp_path, "java_records_test").expect("Failed to copy test data");

    // Search for static method with parameters
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir(temp_path)
        .arg("fromItems")
        .output()
        .expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should find the static method with parameters
    assert!(
        stdout.contains("fromItems"),
        "Should find fromItems static method"
    );

    // Verify the full method signature is visible
    assert!(
        stdout.contains("public static Order fromItems(List<String> items, BigDecimal itemPrice)"),
        "Should show full method signature with parameters"
    );

    // Should contain the record declaration context
    assert!(
        stdout.contains("public record Order"),
        "Should show record declaration context"
    );
}
