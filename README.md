# CodeSearch

A fast code search engine with persistent indexing.

## Features

- Full-text search with persistent indexing
- Gitignore pattern support
- Incremental updates - only reindex changed files
- Simple CLI interface

## Usage

### Basic search
```bash
cargo run -- "search query"
```

### Search in specific directory
```bash
cargo run -- -d /path/to/project "search query"
```

### Rebuild index from scratch
```bash
cargo run -- rebuild
```

### Show index statistics
```bash
cargo run -- stats
```

## Architecture

**Core modules:**
- `file_scanner` - Discovers files with gitignore support
- `search_index` - Full-text search implementation
- `metadata` - File change tracking for incremental updates  
- `search_engine` - Main orchestrator
- `cli` - Command line interface

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Dependencies

- `tantivy` - Full-text search engine
- `ignore` - Gitignore handling
- `clap` - CLI argument parsing
- `serde` + `bincode` - Metadata serialization
- `anyhow` - Error handling