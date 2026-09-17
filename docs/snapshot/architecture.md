# Architecture snapshot

This page describes probe as it exists today. Update it when the module
boundaries or data flow change. Historical rationale lives in
[`../tickets/`](../tickets/), not here.

## What probe is

A local CLI that full-text indexes a project (Tantivy) and optionally reranks
hits with an embedding model. Index state lives under `.probe/` in the project
root. There is no server.

## Module map

| Module | Role |
| --- | --- |
| `src/main.rs` | CLI (`search`, `rebuild`, `stats`, `list-models`, `show-chunks`) |
| `src/search_engine.rs` | Incremental vs rebuild indexing, search + rerank orchestration |
| `src/search_index.rs` | Tantivy schema, document writes, query, snippets |
| `src/reranker.rs` | Optional ML reranking (`fastembed`) and `~/.probe/config.yaml` |
| `src/file_scanner.rs` | `.gitignore`-aware walk; skips known binary extensions |
| `src/code_chunker.rs` | Routes a file to a specialized processor or the fallback indexer |
| `src/language_processor.rs` | `LanguageProcessor` trait + `CodeChunk` |
| `src/languages/java.rs` | AST-aware Java chunking (tree-sitter) |
| `src/languages/fallback.rs` | Whole-file indexer for every other text file |
| `src/metadata.rs` | Incremental file mtime tracking (`metadata.bin`) |
| `src/config.rs` | Project `probe.yml` (stemming) |

## Data flow

```
FileScanner
    │  IndexedFile { disk_path, relative_path }
    ▼
SearchIndex::index_files
    │  skip unreadables, >512KB, lines >8096 bytes
    ▼
CodeChunker::chunk_code_for_indexing
    ├─ specialized LanguageProcessor  (extension match, currently Java)
    └─ FallbackProcessor              (everything else)
    ▼
Tantivy documents (path, declaration, body, filetype, chunk metadata)
    ▼
search → BM25 + field boosts → optional rerank → snippet
```

## Index on disk

- `.probe/` — Tantivy index
- `.probe/metadata.bin` — which files were indexed and when
- Relative paths are stored so the index is portable within the project

## Config surfaces

- Project: `probe.yml` (stemming on/off and language)
- User: `~/.probe/config.yaml` (custom rerankers)
- CLI flags: directory, filetype filter, result count, rerank knobs, context lines

See [`CONFIG.md`](../../CONFIG.md) and [`SCORING_GUIDE.md`](../../SCORING_GUIDE.md)
for the user-facing details of those surfaces.
