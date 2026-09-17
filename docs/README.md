# Docs

This directory is the source of truth for how probe works and why it was built
the way it is. Keep the two kinds of documents separate:

| Kind | Path | When to edit |
| --- | --- | --- |
| **Snapshot** | [`snapshot/`](snapshot/) | Rewrite whenever the current system changes. These pages describe probe *as it is*. |
| **Tickets** | [`tickets/`](tickets/) | Write once, when a feature is built. Do not rewrite later to match later code. |

Root files such as [`README.md`](../README.md), [`CONFIG.md`](../CONFIG.md), and
[`SCORING_GUIDE.md`](../SCORING_GUIDE.md) stay at the repository root so they
remain easy to find. Snapshot pages may link to them; they do not replace them.

## Snapshot

Current-state maps of the code:

- [`snapshot/architecture.md`](snapshot/architecture.md) — modules and data flow
- [`snapshot/indexing.md`](snapshot/indexing.md) — how files become searchable
- [`snapshot/languages.md`](snapshot/languages.md) — specialized vs fallback processors

If you change scanning, chunking, indexing, search, or language support, update
the matching snapshot page in the same change.

## Tickets

Per-feature notes that freeze *why* and *how* at the moment of implementation.
Later snapshot updates do not rewrite tickets; a new ticket can supersede an
old one.

- [`tickets/README.md`](tickets/README.md) — how to add a ticket
- [`tickets/TEMPLATE.md`](tickets/TEMPLATE.md) — copy this
- [`tickets/0001-fallback-indexer.md`](tickets/0001-fallback-indexer.md) — first ticket
