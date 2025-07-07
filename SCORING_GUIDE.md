# Search Query Scoring Guide

This guide explains how to write effective search queries for probe and how the scoring system works.

## Overview

probe uses the Tantivy search library to provide fast, full-text search with relevance scoring. The search engine considers multiple factors when ranking results to show the most relevant code matches first.

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
probe "async^2.0 await^0.5"

# Boost exact phrase matches
probe "error handling^1.5"
```

### Field-Specific Searches

Target specific fields using field prefixes:

```bash
# Search only in file paths
probe "path:src/handlers"

# Search only in content
probe "content:database"

# Combine field searches with boosting
probe "path:test^2.0 OR content:unittest"
```

## Proximity and Phrase Matching

### Exact Phrases

Use quotes for exact phrase matching:

```bash
# Find exact phrase
probe "error handling"

# Find phrases with some flexibility
probe "\"async function\"~2"
```

### Proximity Search

The `~` operator allows words to appear near each other:

```bash
# Allow up to 2 words between "async" and "function"
probe "\"async function\"~2"

# This would match: "async def my_function", "async fn", etc.
```

## Advanced Query Syntax

### Boolean Operators

```bash
# AND (both terms must appear)
probe "database AND connection"

# OR (either term can appear)
probe "error OR exception"

# NOT (exclude terms)
probe "function NOT test"
```

### Wildcards

```bash
# Wildcard matching
probe "handle*"  # matches "handler", "handling", etc.

# Combine with field targeting
probe "path:*.js AND react"
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
probe "function handleError"

# Better: Use field boosting for precision
probe "path:error^2.0 function^1.5"

# Best: Combine techniques
probe "\"error handling\"~1 OR path:exception^2.0"
```

### For Finding Definitions

```bash
# Find class definitions
probe "class UserManager"

# Find function definitions with boosting
probe "\"function connect\"^2.0 OR \"def connect\"^2.0"
```

### For Code Patterns

```bash
# Find async patterns
probe "\"async def\"~1 OR \"async function\"~1"

# Find error handling patterns
probe "\"try catch\"~2 OR \"except\"^1.5"
```

## Query Examples

### Finding Functions
```bash
# Find all functions named "validate"
probe "function validate OR def validate"

# Find validation functions with boosting
probe "\"validate\"^2.0 AND (function OR def)"
```

### Finding Imports/Dependencies
```bash
# Find imports of specific modules
probe "import react OR from react"

# Find require statements
probe "require('express') OR require(\"express\")"
```

### Finding Test Code
```bash
# Find test functions
probe "path:test^2.0 AND (function OR def)"

# Find specific test patterns
probe "\"it should\"~2 OR \"test_\"^1.5"
```

## Tips

1. **Start simple**: Begin with basic terms, then add complexity
2. **Use quotes**: For exact phrases or code snippets
3. **Boost important terms**: Use `^` to emphasize key terms
4. **Target fields**: Use `path:` for file-based searches
5. **Test proximity**: Use `~` for flexible phrase matching

The scoring system is designed to surface the most relevant code matches based on where terms appear and how they're structured in your codebase.

## Reranking for Better Results

probe includes AI-powered reranking that improves search relevance by re-ordering results based on semantic similarity to your query.

### How Reranking Works

1. **Initial Search**: Tantivy finds candidate results (typically 10+ matches)
2. **Reranking**: AI model scores each result for semantic relevance to your query
3. **Final Results**: Results are re-ordered by AI relevance scores

### Reranking Options

```bash
# Reranking is enabled by default
probe "error handling"

# Disable reranking for faster searches
probe --no-rerank "error handling"

# Use a different reranking model
probe --rerank-model jina-reranker-v2-base-multilingual "error handling"

# Adjust candidate pool size (more candidates = better reranking)
probe --rerank-candidates 20 "error handling"
```

### Available Reranking Models

```bash
# List all available models
probe list-models
```

- **bge-reranker-base**: Default, good balance of speed and accuracy
- **bge-reranker-v2-m3**: Improved version with better multilingual support
- **jina-reranker-v1-turbo-en**: Fast English-only model
- **jina-reranker-v2-base-multilingual**: Best for multilingual codebases

### When Reranking Helps Most

- **Complex queries**: Multi-word or conceptual searches
- **Large result sets**: When many files match your query
- **Semantic similarity**: Finding code that does similar things but uses different terms

### When to Disable Reranking

- **Simple exact matches**: Single-word or exact phrase searches
- **Speed priority**: When you need results immediately
- **Limited resources**: On slower machines or with large codebases