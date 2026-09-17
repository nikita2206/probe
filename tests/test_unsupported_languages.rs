use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_data/corpus");

struct Corpus {
    dir: TempDir,
}

impl Corpus {
    /// Copy the shared mixed-type tree and rebuild.
    fn load() -> Self {
        let dir = TempDir::new().unwrap();
        copy_dir(Path::new(CORPUS), dir.path()).unwrap();
        rebuild(dir.path());
        Self { dir }
    }

    /// Same tree, plus oversized / long-line files that the indexer must skip.
    fn load_with_skip_files() -> Self {
        let dir = TempDir::new().unwrap();
        copy_dir(Path::new(CORPUS), dir.path()).unwrap();
        materialize_skip_files(dir.path());
        rebuild(dir.path());
        Self { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn search(&self, query: &str, extra: &[&str]) -> (String, String) {
        let mut args = vec![query, "--no-rerank"];
        args.extend_from_slice(extra);
        let output = Command::cargo_bin("probe")
            .unwrap()
            .current_dir(self.path())
            .args(&args)
            .assert()
            .success();
        (
            String::from_utf8_lossy(&output.get_output().stdout).into_owned(),
            String::from_utf8_lossy(&output.get_output().stderr).into_owned(),
        )
    }

    fn ls_files(&self) -> String {
        let output = Command::cargo_bin("probe")
            .unwrap()
            .current_dir(self.path())
            .args(["stats", "--ls-files"])
            .assert()
            .success();
        String::from_utf8_lossy(&output.get_output().stdout).into_owned()
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn materialize_skip_files(root: &Path) {
    let skip = root.join("skip");
    fs::create_dir_all(&skip).unwrap();
    fs::write(skip.join("too_large.txt"), "x".repeat(600 * 1024)).unwrap();
    fs::write(
        skip.join("minified.js"),
        format!("var x = {};", "a".repeat(9000)),
    )
    .unwrap();
}

fn rebuild(root: &Path) {
    Command::cargo_bin("probe")
        .unwrap()
        .current_dir(root)
        .arg("rebuild")
        .assert()
        .success();
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected `{needle}` in:\n{haystack}"
    );
}

fn paths_named(listed: &str, name: &str) -> Vec<PathBuf> {
    listed
        .lines()
        .map(PathBuf::from)
        .filter(|path| path.file_name().and_then(|n| n.to_str()) == Some(name))
        .collect()
}

#[test]
fn python_function_is_searchable() {
    let corpus = Corpus::load();
    let (stdout, _) = corpus.search("multiply_numbers", &[]);
    assert_contains(&stdout, "calculator.py");
}

#[test]
fn javascript_function_is_searchable_with_context() {
    let corpus = Corpus::load();
    let (stdout, stderr) = corpus.search("truncateString", &["-C", "2"]);
    assert_contains(&stderr, "Found");
    assert_contains(&stdout, "utils.js");
}

#[test]
fn shared_token_hits_python_ruby_and_go() {
    let corpus = Corpus::load();
    let (stdout, _) = corpus.search("crossfile_hit", &["-n", "10"]);
    assert_contains(&stdout, "calculator.py");
    assert_contains(&stdout, "script.rb");
    assert_contains(&stdout, "main.go");
}

#[test]
fn markdown_yaml_and_extensionless_files_are_searchable() {
    let corpus = Corpus::load();
    let (stdout, _) = corpus.search("globalsearchtoken", &["-n", "10"]);
    assert_contains(&stdout, "NOTES.md");
    assert_contains(&stdout, "service.yaml");
    assert_contains(&stdout, "Makefile");
    assert_contains(&stdout, "Dockerfile");
}

#[test]
fn context_lines_are_taken_from_the_matching_segment() {
    let corpus = Corpus::load();
    let (stdout, stderr) = corpus.search("TARGET", &["-C", "1"]);
    assert_contains(&stdout, "notes.txt");
    assert!(
        stderr.contains("TARGET") || stdout.contains("TARGET"),
        "expected TARGET in output:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
}

#[test]
fn java_uses_the_specialized_indexer_in_a_mixed_tree() {
    let corpus = Corpus::load();

    let (java_stdout, _) = corpus.search("sayHello", &[]);
    assert_contains(&java_stdout, "Greeter.java");

    let (text_stdout, _) = corpus.search("plainfallbacknote", &[]);
    assert_contains(&text_stdout, "notes.txt");
}

#[test]
fn oversized_and_minified_files_are_skipped() {
    let corpus = Corpus::load_with_skip_files();
    let listed = corpus.ls_files();

    assert_contains(&listed, "calculator.py");
    assert_contains(&listed, "utils.js");
    assert!(
        paths_named(&listed, "too_large.txt").is_empty(),
        "too_large.txt should not be indexed:\n{listed}"
    );
    assert!(
        paths_named(&listed, "minified.js").is_empty(),
        "minified.js should not be indexed:\n{listed}"
    );
}
