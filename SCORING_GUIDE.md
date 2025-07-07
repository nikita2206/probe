# Search Query Scoring Guide

This guide explains how to write effective search queries for codesearch-rs and how the scoring system works.

## Overview

codesearch-rs uses the Tantivy search library to provide fast, full-text search with relevance scoring. The search engine considers multiple factors when ranking results to show the most relevant code matches first.

## Field Boosting

### Basic Field Weights

The search engine automatically applies different weights to different parts of your code:

- **Function/class names**: 3x weight (highest priority)
- **File paths**: 2x weight (moderate priority)  
- **File content**: 1x weight (baseline)

This means matches in function names and class definitions will rank higher than matches in comments or regular code.

### Query-Time Boosting

You can boost specific terms in your search using the `^` operator:

```bash
# Boost "async" term 2x, "await" term 0.5x
codesearch-rs "async^2.0 await^0.5"

# Boost exact phrase matches
codesearch-rs "error handling^1.5"
```

### Field-Specific Searches

Target specific fields using field prefixes:

```bash
# Search only in file paths
codesearch-rs "path:src/handlers"

# Search only in content
codesearch-rs "content:database"

# Combine field searches with boosting
codesearch-rs "path:test^2.0 OR content:unittest"
```

## Proximity and Phrase Matching

### Exact Phrases

Use quotes for exact phrase matching:

```bash
# Find exact phrase
codesearch-rs "error handling"

# Find phrases with some flexibility
codesearch-rs "\"async function\"~2"
```

### Proximity Search

The `~` operator allows words to appear near each other:

```bash
# Allow up to 2 words between "async" and "function"
codesearch-rs "\"async function\"~2"

# This would match: "async def my_function", "async fn", etc.
```

## Advanced Query Syntax

### Boolean Operators

```bash
# AND (both terms must appear)
codesearch-rs "database AND connection"

# OR (either term can appear)
codesearch-rs "error OR exception"

# NOT (exclude terms)
codesearch-rs "function NOT test"
```

### Wildcards

```bash
# Wildcard matching
codesearch-rs "handle*"  # matches "handler", "handling", etc.

# Combine with field targeting
codesearch-rs "path:*.js AND react"
```

## Scoring Factors

The search engine considers several factors when ranking results:

1. **Term frequency**: How often the search term appears
2. **Field importance**: Function names > file paths > content
3. **Document length**: Shorter matches may rank higher
4. **Proximity**: Terms closer together score higher
5. **Exact matches**: Exact phrase matches get bonus points

## Best Practices

### For Code Search

```bash
# Good: Target specific code constructs
codesearch-rs "function handleError"

# Better: Use field boosting for precision
codesearch-rs "path:error^2.0 function^1.5"

# Best: Combine techniques
codesearch-rs "\"error handling\"~1 OR path:exception^2.0"
```

### For Finding Definitions

```bash
# Find class definitions
codesearch-rs "class UserManager"

# Find function definitions with boosting
codesearch-rs "\"function connect\"^2.0 OR \"def connect\"^2.0"
```

### For Code Patterns

```bash
# Find async patterns
codesearch-rs "\"async def\"~1 OR \"async function\"~1"

# Find error handling patterns
codesearch-rs "\"try catch\"~2 OR \"except\"^1.5"
```

## Query Examples

### Finding Functions
```bash
# Find all functions named "validate"
codesearch-rs "function validate OR def validate"

# Find validation functions with boosting
codesearch-rs "\"validate\"^2.0 AND (function OR def)"
```

### Finding Imports/Dependencies
```bash
# Find imports of specific modules
codesearch-rs "import react OR from react"

# Find require statements
codesearch-rs "require('express') OR require(\"express\")"
```

### Finding Test Code
```bash
# Find test functions
codesearch-rs "path:test^2.0 AND (function OR def)"

# Find specific test patterns
codesearch-rs "\"it should\"~2 OR \"test_\"^1.5"
```

## Tips

1. **Start simple**: Begin with basic terms, then add complexity
2. **Use quotes**: For exact phrases or code snippets
3. **Boost important terms**: Use `^` to emphasize key terms
4. **Target fields**: Use `path:` for file-based searches
5. **Test proximity**: Use `~` for flexible phrase matching

The scoring system is designed to surface the most relevant code matches based on where terms appear and how they're structured in your codebase.