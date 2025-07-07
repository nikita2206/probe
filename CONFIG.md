# Probe Configuration

This document explains how to configure probe's search behavior and reranking models.

## Project Configuration (probe.yml)

Probe supports per-project configuration via a `probe.yml` file in the project root.

### Stemming Configuration

```yaml
stemming:
  enabled: true
  language: english
```

**Supported Languages:**
- english / en (default)
- french / fr, german / de, italian / it, portuguese / pt, spanish / es
- dutch / nl, danish / da, finnish / fi, hungarian / hu, norwegian / no
- romanian / ro, russian / ru, swedish / sv, tamil / ta, turkish / tr

**Behavior:**
- When enabled: Queries are stemmed to match word variations (e.g., "run" matches "running", "runs")
- When disabled: Only exact word matches are found
- Default: Enabled with English language

## User Configuration (~/.probe/config.yaml)

Global user configuration for reranking models and preferences. Default location: `~/.probe/config.yaml` (override with `--config` flag).

### Custom Reranker Configuration

```yaml
# Optional: Default reranker to use when --rerank-model is not specified
default_reranker: "model-name"

custom_rerankers:
  model-name:
    description: "Human-readable description of the model"
    model_code: "huggingface/model-repo-path"
    model_file: "model.onnx"
    additional_files:
      - "model.onnx.data"
      - "other-file.bin"
```

## Configuration Fields

- **default_reranker**: (Optional) The name of the custom reranker to use by default when `--rerank-model` is not specified
- **model-name**: A unique identifier for the model (used with `--rerank-model`)
- **description**: Human-readable description of the model
- **model_code**: The HuggingFace repository path (e.g., "BAAI/bge-reranker-large")
- **model_file**: The main ONNX model file name (usually "model.onnx" or "pytorch_model.onnx")
- **additional_files**: List of additional files required by the model (e.g., "model.onnx.data" for large models)

## Example Configuration

```yaml
# Default reranker to use when --rerank-model is not specified
default_reranker: "bge-reranker-v2-m3-quant"

custom_rerankers:
  bge-reranker-v2-m3-quant:
    description: "ONNX version of Quantized model of bge-reranker-v2-m3"
    model_code: "sudhanshu746/bge-reranker-v2-m3-quant-onnx"
    model_file: "model.onnx"
    additional_files:
      - "model.onnx.data"
  
  bge-reranker-large:
    description: "BAAI BGE reranker large model"
    model_code: "BAAI/bge-reranker-large"
    model_file: "model.onnx"
    additional_files: []
  
  ms-marco-minilm:
    description: "Cross-encoder model trained on MS MARCO"
    model_code: "cross-encoder/ms-marco-MiniLM-L-12-v2"
    model_file: "pytorch_model.onnx"
    additional_files: []
```

## Usage

The `--rerank-model` option accepts both built-in model names and custom model names from your config file.

### Using Default Config Location

```bash
# Create ~/.probe/config.yaml with your custom models

# Use default reranker specified in config file
probe "search query"

# Override with specific custom model
probe --rerank-model "bge-reranker-v2-m3-quant" "search query"

# Override with built-in model
probe --rerank-model "bge-reranker-base" "search query"
```

### Using Custom Config Location

```bash
# Use default reranker from custom config file
probe --config /path/to/my-config.yaml "search query"

# Override with specific custom model
probe --config /path/to/my-config.yaml --rerank-model "bge-reranker-v2-m3-quant" "search query"

# Override with built-in model
probe --config /path/to/my-config.yaml --rerank-model "bge-reranker-base" "search query"
```

## Required Files

The following files are automatically downloaded based on your configuration:

1. **Main model file**: Specified in `model_file` field
2. **Additional files**: All files listed in `additional_files` array
3. **Tokenizer files**: `tokenizer.json`, `special_tokens_map.json`, `tokenizer_config.json`
4. **Config file**: `config.json`

## Notes

- The configuration approach is deterministic - you specify exactly which files to download
- No automatic file discovery is performed
- If a required file is missing from the HuggingFace repository, the download will fail with a clear error message
- Models are cached locally after first download for faster subsequent use

