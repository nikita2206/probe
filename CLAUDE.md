# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**probe** is a Rust-based smart code search tool that combines full-text search with ML-powered reranking. It's designed to work like grep but with better relevance scoring using embedding models.

## Core Architecture

### Main Components
- **SearchEngine** (`src/search_engine.rs`): Main orchestrator, handles indexing and search operations
- **SearchIndex** (`src/search_index.rs`): Tantivy-based full-text search index
- **Reranker** (`src/reranker.rs`): AI-powered result reranking using fastembed embeddings
- **CodeChunker** (`src/code_chunker.rs`): Splits files into searchable chunks 
- **FileScanner** (`src/file_scanner.rs`): Discovers files while respecting .gitignore
- **LanguageProcessor** (`src/language_processor.rs`): Language-specific code parsing

### Key Design Patterns
- Index stored in `.probe/` directory in project root
- Metadata tracking in `metadata.bin` for incremental updates
- Language-specific processors in `src/languages/` (currently Java)
- Configuration via `probe.yml` (project) and `~/.probe/config.yaml` (user)

## Development Commands

### Building and Testing
```bash
# Build (release)
cargo build --release

# Build (debug)
cargo build

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_tests
```

### Code Quality
```bash
# Check formatting
cargo fmt -- --check

# Auto-fix formatting
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check with verbose output
cargo build --verbose
cargo test --verbose
```

### Running the Tool
```bash
# Basic search in current directory
cargo run -- "search query"

# Search with specific options
cargo run -- -d /path/to/dir --rerank-model bge-reranker-base "query"

# Rebuild index
cargo run -- rebuild

# Show statistics
cargo run -- stats

# List available models
cargo run -- list-models
```

## Testing Strategy

- Unit tests in each module
- Integration tests in `tests/` directory with test data
- CI runs formatting, clippy, build, and tests
- Test data includes Java examples for language processing

## Configuration System

- Project config: `probe.yml` (stemming settings)
- User config: `~/.probe/config.yaml` (custom reranking models)
- Built-in reranking models via fastembed library
- Custom models downloaded from HuggingFace

## Key Dependencies

- **tantivy**: Full-text search engine
- **fastembed**: Embedding models for reranking
- **tree-sitter**: Code parsing (Java support via tree-sitter-java)
- **ignore**: .gitignore handling
- **clap**: CLI argument parsing