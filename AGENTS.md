# AGENTS.md

This file provides guidance to coding agents working in this repository.

## Project Overview

`probe` is a Rust-based code search tool. It combines a Tantivy full-text index with optional ML-powered reranking to improve relevance over plain grep-style matching.

## Core Architecture

### Main Components
- `src/search_engine.rs`: high-level orchestration for indexing and search
- `src/search_index.rs`: Tantivy index creation, updates, and query execution
- `src/reranker.rs`: optional reranking configuration and embedding-based reranking
- `src/code_chunker.rs`: splits files into searchable chunks
- `src/file_scanner.rs`: file discovery with `.gitignore` handling
- `src/language_processor.rs`: abstraction for language-aware parsing
- `src/languages/java.rs`: current Java-specific chunk extraction

### Key Design Notes
- Index state lives under `.probe/` in the target project root.
- Incremental indexing metadata is stored in `metadata.bin`.
- Project config is read from `probe.yml`.
- User config is read from `~/.probe/config.yaml`.
- Java is the only language with AST-aware chunking today; unsupported languages fall back to plain text indexing with context lines.

## Development Commands

### Build
```bash
cargo build
cargo build --release
```

### Test
```bash
# Full suite
cargo test

# Show test output
cargo test -- --nocapture

# One integration test target
cargo test --test integration_tests

# One specific test
cargo test test_basic_search -- --nocapture

# Java chunking unit tests
cargo test java_test -- --nocapture
```

### Quality Checks
```bash
cargo fmt
cargo fmt -- --check
cargo clippy -- -D warnings
```

### Run the CLI
```bash
cargo run -- "search query"
cargo run -- rebuild
cargo run -- stats
cargo run -- list-models
cargo run -- -d /path/to/project --no-rerank "query"
```

## Implementation Rules

### Tests Are Mandatory
- Any behavior change, bug fix, parser change, indexing change, CLI change, or ranking change should come with test coverage unless there is a concrete reason it cannot.
- Prefer adding or updating the test that exercises the user-visible behavior you are changing instead of relying on manual inspection.
- If a change affects indexing, parsing, snippets, filters, config loading, or CLI output, assume a regression test is needed.

### Use a Short Feedback Loop
When implementing:
1. Identify the narrowest existing test that covers the area, or add one first.
2. Run that narrow test before changing code to confirm the baseline or reproduce the bug.
3. Make the smallest code change that should fix the issue.
4. Rerun the same narrow test immediately.
5. If it passes, run the next wider relevant set of tests.
6. Before finishing, run at least the relevant integration test target if the change affects end-to-end behavior.

Do not batch large unverified edits and only test at the end.

## Current Test Setup

### Test Layout
- `src/languages/tests/java_test.rs`: unit tests for Java chunk extraction and declaration/content boundaries
- `tests/integration_tests.rs`: end-to-end CLI coverage using `assert_cmd`, temporary directories, and copied fixture projects
- `tests/test_java_records.rs`: integration-style regression tests for Java record search behavior
- `tests/test_java_interface_methods.rs`: integration-style regression tests for Java interface method indexing/search
- `tests/test_unsupported_languages.rs`: fallback indexing/search behavior for unsupported languages such as Python and JavaScript
- `tests/test_stemming_and_config.rs`: direct `SearchEngine` coverage for stemming and config loading
- `tests/test_search_query_processing.rs`: direct `SearchIndex` coverage for snippet generation and query parsing behavior
- `tests/test_data/`: fixture projects and source files copied into temp directories during tests

### Integration Test Details
- Integration tests are standard Rust integration tests in `tests/`.
- Most CLI tests use `assert_cmd::Command::cargo_bin(...)` to execute the built binary.
- Fixture directories are copied into `tempfile::TempDir` so tests can rebuild indexes and mutate files without touching the repository.
- `tests/integration_tests.rs` covers rebuilds, search results, `.gitignore`, incremental updates, `stats`, help output, and filetype filtering.
- Java-specific integration regressions live in dedicated files rather than in the main integration target.

### How To Choose What To Run
- Parser or chunking changes: run `cargo test java_test -- --nocapture` and the affected Java integration tests.
- Search/index behavior changes: run the relevant test in `tests/integration_tests.rs` plus `cargo test --test integration_tests`.
- Snippet/query logic changes: run `cargo test test_search_query_processing -- --nocapture`.
- Config or stemming changes: run `cargo test test_stemming_and_config -- --nocapture`.
- Unsupported language fallback changes: run `cargo test test_unsupported_languages -- --nocapture`.
- Before finalizing a non-trivial change: run `cargo test`.

## Dependencies

- `tantivy`: full-text indexing and search
- `fastembed`: reranking models
- `tree-sitter` and `tree-sitter-java`: parsing and Java AST traversal
- `ignore`: `.gitignore`-aware scanning
- `clap`: CLI parsing
