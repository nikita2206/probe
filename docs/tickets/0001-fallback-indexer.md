# Ticket 0001: Fallback indexer for unspecialized file types

- Status: implemented
- Date: 2026-09-12
- Related: #6, #5; `src/languages/fallback.rs`, `src/code_chunker.rs`

## Why

Probe’s only specialized indexer is Java (tree-sitter AST chunks). Real
projects are mixed: Python, JS, Go, YAML, Markdown, Makefiles, Dockerfiles,
extensionless scripts. Before this ticket the catch-all lived as an inline
branch in `CodeChunker`. That worked for a few tests, but:

1. It was not a `LanguageProcessor`, so it could not be reasoned about,
   tested, or swapped the same way as Java.
2. There was no explicit rule that *specialized processors always win* and
   *everything else still gets indexed*.
3. Issue #6 asked for the tool to be usable on unsupported languages via a
   less optimal chunking strategy.

The user-visible requirement is: **any text file the scanner accepts must be
searchable**, even when no language-specific indexer exists.

## How

A first-class `FallbackProcessor` implements `LanguageProcessor`:

- `get_file_extensions()` returns empty. The fallback is never registered in
  the extension map, so it cannot collide with Java (or a future processor).
- `chunk_code` returns no chunks for empty/whitespace content; otherwise one
  `ChunkType::Other` chunk named `"file"` covering the whole file
  (`start_line = 0`, `end_line = last line`).
- `CodeChunker` holds specialized processors in a `HashMap<ext, processor>`
  plus a single `fallback: Box<dyn LanguageProcessor>`. Lookup is: extension
  match → specialized; else → fallback. Extensionless paths go to fallback.

Search already treats `Other` / `file` chunks as whole-file documents and
extracts a highlighted window with `-C` context lines. That path is reused
rather than inventing a second snippet strategy.

Binary skip rules in `FileScanner` are unchanged (images, archives, `pdf`,
native libs, …). “Any file type” means any *text* file the scanner does not
reject, not binary blobs.

Invariants locked by tests:

- `.java` files produce Class/Method chunks, never a whole-file `"file"` chunk
- `.py`, `.js`, `.md`, `Makefile` produce a single fallback chunk
- CLI search finds Markdown, YAML, Makefile, and Dockerfile contents
- Java method search still works in a mixed Java + text project

## Decisions

- Decision: whole-file chunk, no sliding windows.
  Alternatives considered: line-window / paragraph chunks; generic tree-sitter
  fallback; skip unknown types.
  Why this one: issue #6 explicitly allowed “no chunking at all”; search-time
  context extraction already exists; windowing would change scoring and
  snippets without a clear quality target. Revisit if large-file BM25 dilution
  becomes a real complaint.

- Decision: fallback is a `LanguageProcessor`, not a one-off in `SearchIndex`.
  Alternatives considered: keep the inline `CodeChunker` branch; add a second
  indexing pipeline.
  Why this one: one routing point (`CodeChunker`) so `show-chunks`, unit tests,
  and indexing all share the same rule.

- Decision: specialized processors always win on their extensions, even if
  they return no useful chunks.
  Alternatives considered: fall back when the specialized parser errors or
  returns empty.
  Why this one: silent fallback would hide parser bugs and mix AST chunks with
  whole-file chunks for the same language. Empty specialized output still
  reaches `SearchIndex`, which already stores the whole file when `chunks` is
  empty.

## Follow-ups

- Specialized processors for Python / JS / TS / Go / Rust (#5) should register
  in `CodeChunker::new()` and immediately take those extensions away from
  fallback. No fallback change required.
- If whole-file BM25 on large sources is weak, add windowed fallback chunking
  behind the same processor rather than a new pipeline.
- NUL-byte / better binary detection is still open (architecture notes on #31);
  fallback does not try to index binary files.
