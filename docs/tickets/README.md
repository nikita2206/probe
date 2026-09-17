# Feature tickets

Tickets capture **why** and **how** a feature was built, frozen at the time of
implementation. They are not living documentation.

- When the system later changes, **update the snapshot**, do not rewrite the
  ticket to match the new code.
- If a later change supersedes a ticket, add a new ticket and point at the old
  one. Leave the original text in place.
- Number tickets monotonically: `0001-…`, `0002-…`.
- Copy [`TEMPLATE.md`](TEMPLATE.md) for a new ticket.

A ticket is required for any change that introduces a new user-visible
capability, a new processor, a scoring/indexing policy, or a CLI command.
Small bug fixes do not need a ticket unless they encode a lasting design
choice.

## Index

| ID | Title | Status |
| --- | --- | --- |
| [0001](0001-fallback-indexer.md) | Fallback indexer for unspecialized file types | implemented |
