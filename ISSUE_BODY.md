## Summary
- **Separation of concerns is solid** across scanning, chunking, indexing, reranking, and CLI.
- **Search quality is good** with field boosts and ML reranking; Java chunking adds structure.
- **Clear opportunities** to tighten index schema, blend scores, expand languages, and streamline UX.

### Architecture overview
- `SearchEngine`: Orchestrates incremental indexing (`.probe/metadata.bin`), rebuilds, and high-level search with optional reranking.
- `SearchIndex` (tantivy): Defines schema, tokenization, indexing (parallel chunking), searching, snippets, and simple score penalties.
- `Reranker` (fastembed): Loads built‑in or custom HF models; reranks documents, returning ML scores.
- `CodeChunker` + `LanguageProcessor` (tree‑sitter): Java parser splits classes/methods with declaration/body context and line ranges.
- `FileScanner` (ignore): Walks repo respecting .gitignore; filters binaries by extension.
- `Config`: Project config (`probe.yml`) for stemming and language; user config (`~/.probe/config.yaml`) for custom rerankers.
- `CLI`: Rebuild/stats/list-models/show-chunks and search with rerank knobs.

### Strengths
- **Modular design**: Each responsibility lives in a focused module; easy to extend.
- **Incremental indexing** with metadata and a clean rebuild path.
- **Language-aware chunking** for Java yields semantically meaningful results and better snippets.
- **ML reranking** with support for custom Hugging Face models.
- **Reasonable defaults** for field boosts (declaration > names > body) and snippet highlighting.

### Improvement opportunities and recommendations
- **Index schema and query**
  - Use exact, non-tokenized fields for `filetype`, `chunk_type`, and `path` to make filters precise and faster; keep a separate tokenized `path_components` for partial matches if needed.
  - Normalize and store paths relative to the project root to improve portability and output readability.
  - Consider storing less in the index: avoid storing full `body` for large chunks; re-read from file for snippets or store compact windows around declarations.

- **Scoring and reranking**
  - Blend BM25 and reranker scores instead of replacing: `final = α·normalize(bm25) + (1−α)·normalize(ml)` with configurable `α`.
  - Move hand-tuned penalties (tests/classes) into a configurable scoring policy (YAML), and add path-aware boosts (e.g., prioritize `src/main`, down-rank `vendor`/generated).
  - Add optional freshness/churn signals via git metadata (recently modified, frequently edited) as boosts.
  - Truncate or summarize documents sent to the reranker (declaration + leading lines or the most relevant spans) to reduce latency and improve signal.

- **Chunking and languages**
  - Add processors for TS/JS, Python, Rust, and Go; keep Java path as a template. Provide a generic tree-sitter fallback.
  - Add chunk-size controls (merge small methods, split oversized chunks) to balance recall vs noise.
  - Persist stable chunk IDs and offsets to enable editor integrations and precise navigation.

- **Index lifecycle and metadata**
  - Record an explicit schema/version and relevant config (e.g., stemming) in metadata; auto-rebuild if it changes to avoid compatibility branches.
  - Handle deletions: remove docs for files that no longer exist (tombstones or delete-by-path map).

- **Performance and concurrency**
  - Batch adds and commit by configurable batch size; tune Tantivy merge policy for many small docs.
  - Separate bounded pools for parsing/chunking vs indexing to avoid oversubscription; expose `--index-threads`.
  - Optional daemon/watch mode to keep index hot for large repos.

- **CLI and UX**
  - Unify default reranker: library default is BGE; CLI fallback currently resolves to JINA in several branches—pick one source of truth and document it.
  - Add `--json` output including `path`, `score`, `chunk_type`, `chunk_name`, `start_line`, `end_line` for tool/IDE integrations.
  - Add `--path` prefix filter and `--ignore` globs to complement .gitignore.
  - Enhance `show-chunks` with `--json` and line numbers; add `--open` to launch `$EDITOR +{line} {file}`.

- **Reliability and correctness**
  - Improve binary detection beyond extension (quick NUL-byte scan) and add a `max_file_size` guard.
  - Cap snippet size and highlight a small window around matches for very large functions.

- **Testing and quality**
  - Property tests for chunker (balanced braces, nested classes, comments, javadoc placement).
  - Integration tests: schema migration, deletion handling, rerank blending, path filters, multi-language processors.
  - Structured logging via `tracing` with `RUST_LOG=probe=info` and model download stubbing in tests.

- **Extensibility**
  - Language processor registry behind feature flags to slim binaries.
  - Optional lightweight HTTP/gRPC service to expose JSON search for editors.

### Quick wins
- Switch `filetype`/`chunk_type`/`path` to exact non-tokenized fields and adjust query building.
- Normalize relative paths in outputs and index.
- Externalize penalties/boosts to config.
- Add `--json` output and `--path` filter.
- Unify default reranker selection in one place.

### Deeper enhancements
- Score blending with configurable weights and extra features (recency, path boosts).
- Schema versioning with auto-rebuild and deletion handling.
- Multi-language processors and watcher-based continuous indexing.

### Proposed next steps
- [ ] Align reranker defaults and document model choices.
- [ ] Convert fields to exact-match where appropriate and add `--json` output.
- [ ] Introduce config-driven scoring policy and remove hard-coded penalties.
- [ ] Add deletion handling and schema/version tracking with auto-rebuild.
- [ ] Plan multi-language support (prioritize TS/JS and Python) and chunk-size controls.
