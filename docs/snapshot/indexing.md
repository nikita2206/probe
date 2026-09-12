# Indexing snapshot

How a file becomes searchable. Update this page when scanning, skip rules,
chunk routing, or the Tantivy schema change.

## Discovery

`FileScanner` walks the project with `ignore::WalkBuilder`:

- Respects `.gitignore`, global gitignore, and `.git/info/exclude`
- Always skips `.probe/` and `.git/`
- Includes hidden files
- Drops known binary extensions (`exe`, `dll`, images, archives, `pdf`, …)
- Files with no extension *are* candidates (Makefile, Dockerfile, …)

A file that survives the walk is still not guaranteed into the index. The
indexer then skips:

- Content that is not valid UTF-8 (`fs::read_to_string` fails)
- Files larger than 512 KiB
- Files with any line longer than 8096 bytes

Those skip rules are silent. Incremental indexing uses `metadata.bin` mtimes;
`rebuild` wipes `.probe/` and starts over.

## Chunk routing

`CodeChunker` picks a processor from the file extension:

1. If a specialized `LanguageProcessor` registered that extension, use it.
2. Otherwise use `FallbackProcessor`.

Java is the only specialized processor today (`.java`). Fallback owns every
other text file, including unknown extensions and extensionless names.

The fallback writes one `ChunkType::Other` chunk named `"file"` covering the
whole contents. Search then extracts a highlighted window with `-C` / `--context`
lines rather than showing the entire file.

## Schema (per chunk)

| Field | Indexed? | Notes |
| --- | --- | --- |
| `path` | stored | Relative to project root |
| `declaration` | camel-case tokenizer, stored | Boost 3.0 |
| `body` | camel-case tokenizer, stored | Boost 1.0 |
| `chunk_name` | camel-case tokenizer, stored | Boost 2.5 |
| `filetype` | text + stored | Used by `-t` / `--filetype` |
| `chunk_type` | text + stored | `Method`, `Class`, `Other`, … |
| `start_line` / `end_line` | stored u64 | 0-based |

Stemming on the camel-case tokenizer follows `probe.yml`.

## Search-time extras

- Test paths (`*test*` in the path) are down-ranked 50%
- `Class` / `Interface` / `Struct` chunks are down-ranked 40%
- Method/function hits show declaration + body with highlights
- Fallback (`Other` / `file`) hits show the matching segment plus context lines
