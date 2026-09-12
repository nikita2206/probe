# Language processors snapshot

How probe decides *which* indexer handles a file. Update this page when a
processor is added, removed, or when fallback chunking changes.

## Contract

Every indexer implements `LanguageProcessor`:

- `get_file_extensions()` — extensions this processor owns (no leading dot)
- `chunk_code(content)` — `CodeChunk`s to store
- `clone_box()` — per-thread copy used during parallel indexing

`CodeChunker` holds a map of extension → specialized processor, plus a single
fallback processor that is never selected by extension.

## Specialized processors

| Processor | Extensions | Chunking |
| --- | --- | --- |
| `JavaProcessor` | `java` | tree-sitter: classes, interfaces, records, methods, constructors |

Adding a language means: implement `LanguageProcessor`, register it in
`CodeChunker::new()`, add unit tests under `src/languages/tests/`, and update
this table.

## Fallback indexer

`FallbackProcessor` (`src/languages/fallback.rs`) is used when no specialized
processor owns the extension (or the file has no extension).

- Empty / whitespace-only content → no chunks
- Otherwise one `ChunkType::Other` chunk named `"file"` spanning the whole file
- `get_file_extensions()` is empty on purpose so it cannot steal an extension
  from a specialized processor

This is what makes “search any (text) file type” work today: Python, JS, Go,
Ruby, Markdown, YAML, JSON, Makefiles, Dockerfiles, and anything else that
survives the scanner’s binary skip list.

Specialized processors always win on their extensions. A `.java` file is never
sent to fallback, even if the Java parser extracts nothing useful.

## Tests that lock this in

- `src/languages/tests/fallback_test.rs` — processor + routing
- `tests/test_unsupported_languages.rs` — CLI search across fallback file types
