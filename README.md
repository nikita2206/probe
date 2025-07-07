# 🔎 probe (pb)

**probe** is a smart code search tool that works like grep but with better results. It runs locally on your machine with no servers or dependencies, using full-text search with embedding-based reranking to find what you're actually looking for.

## Features

- **Full-text search** - Better relevance scoring than simple text matching
- **ML-powered reranking** - Language model embeddings boost relevant results
- **Works offline** - No servers, APIs, or network dependencies
- **Respects gitignore** - Skips ignored files and binary files automatically
- **Fast updates** - Only reindex changed files
- **Cross-platform** - Works on Linux, macOS, and Windows
- **Zero setup** - Works out of the box in any directory

## Quick Start

```bash
# Search in current directory
probe "snippet generation"

# Search in specific directory
probe -d /path/to/project "cli argument parsing"

# Rebuild index (rarely needed)
probe rebuild

# Show index statistics
probe stats
```

## Installation

### Pre-built binaries

Download the latest release for your platform from the [releases page](https://github.com/nikita2206/probe/releases).

### From source

```bash
git clone https://github.com/nikita2206/probe
cd probe
cargo build --release
```

The binary will be available at `target/release/probe`.

### Using Cargo

```bash
cargo install --git https://github.com/nikita2206/probe
```

## Why probe?

**Better than grep**: Uses full-text search to rank results by relevance, not just pattern matching. Embedding-based reranking puts the most useful results first.

**Perfect for AI agents**: Great alternative to vector search for AI coding assistants. Works offline, no API calls needed.

**Understands code**: Language model reranking helps find related functions, similar patterns, and contextually relevant matches.

**Just works**: No setup, no servers, no dependencies. Works like grep but smarter.

## Performance

probe combines the speed of full-text search with the intelligence of embedding-based reranking. The full-text search provides fast initial results (pulling at least 10 most-relevant results), while the embedding-based reranking ensures the most relevant matches appear at the top.

## Usage

### Basic Search

```bash
# Search for text in current directory
probe "error handling"

# Complex queries
probe "snippet generation"
```

### Index Management

```bash
# Rebuild index from scratch (automatically detects changes)
probe rebuild

# Show index statistics and file counts
probe stats
```

### Directory Selection

```bash
# Search in specific directory
probe -d /path/to/project "search term"

# Search with custom directory
probe --dir ~/code/my-project "function"
```

## How it works

1. **File Discovery**: Scans directories using the same `.gitignore` logic as Git
2. **Full-text Indexing**: Creates a full-text search index using Tantivy
3. **Embedding Reranking**: Uses language model embeddings to boost contextually relevant results
4. **Smart Results**: Combines full-text search scores with semantic similarity for optimal ranking

The index is stored in a `.codesearch/` directory in your project root and is automatically managed.

For details on search scoring, see [SCORING_GUIDE.md](SCORING_GUIDE.md). For query syntax, see [Tantivy's query documentation](https://docs.rs/tantivy/latest/tantivy/query/index.html).

## Configuration

probe works with zero configuration, but you can customize behavior:

- Respects `.gitignore` files automatically
- Skips binary files and common non-text formats
- Excludes the `.codesearch/` index directory from searches

## Building from Source

```bash
git clone https://github.com/nikita2206/probe
cd probe
cargo build --release
```

### Development

```bash
# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- "search term"

# Check code formatting
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings
```

## License

This project is dual-licensed:

- **MIT License** - Free for personal use, open source projects, and non-commercial use
- **Probe Commercial License** - Required for commercial agentic use (AI assistants, automated tools, commercial services)

**Commercial Agentic Use** includes integrating probe into:
- AI coding assistants or development tools
- Commercial code analysis services  
- Automated AI workflows that are sold or monetized
- AI-powered platforms or products

For commercial agentic use, please contact the project maintainer for licensing terms.

See the [LICENSE](LICENSE) file for complete details.

## Similar Tools

- **ripgrep**: Fast regex search, but results aren't ranked by relevance
- **grep**: Basic pattern matching, no relevance scoring
- **Vector search**: Requires embeddings API, slower, needs network
- **The Silver Searcher (ag)**: Fast text search but simple substring matching

probe gives you smart search results without the complexity of vector databases or API dependencies.
