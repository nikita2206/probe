# Mixed-type search corpus

Checked-in fixture for end-to-end tests. Tests copy this tree into a temp
directory, rebuild the index, and search. They do not create extra source files.

| Path | Role |
| --- | --- |
| `java/Greeter.java` | Specialized Java indexer |
| `python/calculator.py` | Fallback (Python) |
| `javascript/utils.js` | Fallback (JavaScript) |
| `ruby/script.rb` | Fallback (Ruby) |
| `go/main.go` | Fallback (Go) |
| `docs/NOTES.md` | Fallback (Markdown) |
| `config/service.yaml` | Fallback (YAML) |
| `notes.txt` | Fallback (plain text, context-line fixture) |
| `Makefile` | Fallback (extensionless) |
| `Dockerfile` | Fallback (extensionless) |

Oversized / long-line skip files are materialized by the test helper at copy
time so they are not stored in git.

