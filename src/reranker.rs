use anyhow::{Context, Result};
use fastembed::{
    OnnxSource, RerankInitOptions, RerankInitOptionsUserDefined, RerankerModel, TextRerank,
    UserDefinedRerankingModel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Custom reranker model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRerankerModel {
    pub description: String,
    pub model_code: String,
    pub model_file: String,
    pub additional_files: Vec<String>,
}

/// Configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbeConfig {
    pub custom_rerankers: HashMap<String, CustomRerankerModel>,
    #[serde(default)]
    pub default_reranker: Option<String>,
}


impl ProbeConfig {
    /// Load configuration from file, with fallback to default
    pub fn load_from_file(config_path: Option<&PathBuf>) -> Result<Self> {
        let config_path = match config_path {
            Some(path) => path.clone(),
            None => Self::default_config_path()?,
        };

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let config_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: ProbeConfig = serde_yaml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(config)
    }

    /// Get the default configuration file path (~/.probe/config.yaml)
    pub fn default_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;

        Ok(home_dir.join(".probe").join("config.yaml"))
    }


    /// Get a custom reranker model by name
    pub fn get_custom_model(&self, model_name: &str) -> Option<&CustomRerankerModel> {
        self.custom_rerankers.get(model_name)
    }
}

/// Configuration for the reranker
#[derive(Debug, Clone)]
pub struct RerankerConfig {
    pub enabled: bool,
    pub model: RerankerModel,
    pub min_candidates: usize,
    pub show_download_progress: bool,
    pub custom_model: Option<String>,
    pub probe_config: Option<ProbeConfig>,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: RerankerModel::BGERerankerBase,
            min_candidates: 10,
            show_download_progress: false,
            custom_model: None,
            probe_config: None,
        }
    }
}

/// Represents a document to be reranked
#[derive(Debug, Clone)]
pub struct RerankDocument {
    pub content: String,
    pub metadata: HashMap<String, String>,
}

/// Result after reranking
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub documents: Vec<RerankDocument>,
    pub rerank_scores: Vec<f32>,
}

/// Downloads a custom HuggingFace model using configuration and returns the local file paths
fn download_hf_model_sync(
    custom_model: &CustomRerankerModel,
    _cache_dir: &std::path::Path,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    use hf_hub::api::sync::Api;
    use hf_hub::Repo;

    // Create API with cache directory
    let api = Api::new()?;

    let repo = api.repo(Repo::model(custom_model.model_code.clone()));

    // Download custom HuggingFace model
    println!(
        "Downloading custom reranking model: {} ({})",
        custom_model.model_code, custom_model.description
    );

    // Download the main model file
    let model_path = repo.get(&custom_model.model_file).with_context(|| {
        format!(
            "Failed to download {} for model {}",
            custom_model.model_file, custom_model.model_code
        )
    })?;

    // Download additional files specified in config
    for additional_file in &custom_model.additional_files {
        let _additional_path = repo.get(additional_file).with_context(|| {
            format!(
                "Failed to download {} for model {}",
                additional_file, custom_model.model_code
            )
        })?;
    }

    // Download required tokenizer and config files
    let tokenizer_path = repo.get("tokenizer.json").with_context(|| {
        format!(
            "Failed to download tokenizer.json for model {}",
            custom_model.model_code
        )
    })?;

    let config_path = repo.get("config.json").with_context(|| {
        format!(
            "Failed to download config.json for model {}",
            custom_model.model_code
        )
    })?;

    // Download additional tokenizer files that might be needed
    let _special_tokens_path = repo.get("special_tokens_map.json").ok();
    let _tokenizer_config_path = repo.get("tokenizer_config.json").ok();

    Ok((model_path, tokenizer_path, config_path))
}

/// Creates a UserDefinedRerankingModel from downloaded HuggingFace model files
fn create_user_defined_model(
    model_path: PathBuf,
    tokenizer_path: PathBuf,
    config_path: PathBuf,
) -> Result<UserDefinedRerankingModel> {
    use fastembed::TokenizerFiles;
    use std::fs;

    // Read the files into byte vectors as required by TokenizerFiles
    let tokenizer_bytes = fs::read(&tokenizer_path).context("Failed to read tokenizer file")?;
    let config_bytes = fs::read(&config_path).context("Failed to read config file")?;

    // Try to read special tokens map if it exists
    let special_tokens_bytes = {
        let special_tokens_path = tokenizer_path
            .parent()
            .unwrap()
            .join("special_tokens_map.json");
        fs::read(&special_tokens_path).unwrap_or_default()
    };

    // Try to read tokenizer config if it exists
    let tokenizer_config_bytes = {
        let tokenizer_config_path = tokenizer_path
            .parent()
            .unwrap()
            .join("tokenizer_config.json");
        fs::read(&tokenizer_config_path).unwrap_or_default()
    };

    let tokenizer_files = TokenizerFiles {
        tokenizer_file: tokenizer_bytes,
        config_file: config_bytes,
        special_tokens_map_file: special_tokens_bytes,
        tokenizer_config_file: tokenizer_config_bytes,
    };

    Ok(UserDefinedRerankingModel::new(
        OnnxSource::File(model_path),
        tokenizer_files,
    ))
}

/// Reranker wrapper that manages the fastembed reranking model
pub struct Reranker {
    model: Arc<TextRerank>,
    config: RerankerConfig,
}

impl Reranker {
    /// Create a new reranker with the specified configuration
    pub fn new(config: RerankerConfig) -> Result<Self> {
        if !config.enabled {
            // Return a dummy reranker if disabled
            return Ok(Self {
                model: Arc::new(Self::create_dummy_model()?),
                config,
            });
        }

        // Set cache directory to a more appropriate location
        let cache_dir = std::env::var("FASTEMBED_CACHE_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                // Default to a cache directory in the user's cache directory
                dirs::cache_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join("codesearch-fastembed")
            });

        let model = if let Some(custom_model_name) = &config.custom_model {
            // Use custom HuggingFace model
            let probe_config = config.probe_config.as_ref().ok_or_else(|| {
                anyhow::anyhow!("Probe configuration is required when using custom models")
            })?;
            Self::create_custom_model(custom_model_name, &cache_dir, probe_config)?
        } else {
            // Use built-in model
            TextRerank::try_new(
                RerankInitOptions::new(config.model.clone())
                    .with_show_download_progress(config.show_download_progress)
                    .with_cache_dir(cache_dir),
            )
            .context("Failed to initialize reranking model")?
        };

        Ok(Self {
            model: Arc::new(model),
            config,
        })
    }

    /// Create a custom model from HuggingFace using configuration
    fn create_custom_model(
        model_name: &str,
        cache_dir: &std::path::Path,
        probe_config: &ProbeConfig,
    ) -> Result<TextRerank> {
        // Look up the custom model in the config
        let custom_model = probe_config.get_custom_model(model_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Custom model '{}' not found in configuration. Please add it to your config file.",
                model_name
            )
        })?;

        let (model_path, tokenizer_path, config_path) =
            download_hf_model_sync(custom_model, cache_dir)?;

        let user_defined_model =
            create_user_defined_model(model_path, tokenizer_path, config_path)?;

        let options = RerankInitOptionsUserDefined::default();

        TextRerank::try_new_from_user_defined(user_defined_model, options)
            .context("Failed to create custom model from user-defined model")
    }

    /// Create a dummy model for disabled reranker (won't be used)
    fn create_dummy_model() -> Result<TextRerank> {
        // Set cache directory to a more appropriate location for dummy model too
        let cache_dir = std::env::var("FASTEMBED_CACHE_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                // Default to a cache directory in the user's cache directory
                dirs::cache_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join("codesearch-fastembed")
            });

        TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::BGERerankerBase).with_cache_dir(cache_dir),
        )
        .context("Failed to create dummy model")
    }

    /// Rerank documents based on query relevance
    pub fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        limit: Option<usize>,
    ) -> Result<RerankResult> {
        if !self.config.enabled || documents.is_empty() {
            return Ok(RerankResult {
                documents,
                rerank_scores: vec![],
            });
        }

        // Extract document content for reranking
        let doc_contents: Vec<&str> = documents.iter().map(|doc| doc.content.as_str()).collect();

        // Perform reranking
        let rerank_results = self
            .model
            .rerank(query, doc_contents, true, None)
            .context("Failed to rerank documents")?;

        // Sort documents by rerank score (higher is better)
        let mut scored_docs: Vec<(RerankDocument, f32)> = documents
            .into_iter()
            .zip(rerank_results)
            .map(|(doc, result)| (doc, result.score))
            .collect();

        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit if specified
        if let Some(limit) = limit {
            scored_docs.truncate(limit);
        }

        let (reranked_docs, rerank_scores): (Vec<_>, Vec<_>) = scored_docs.into_iter().unzip();

        Ok(RerankResult {
            documents: reranked_docs,
            rerank_scores,
        })
    }
}

/// Parse reranker model from string
pub fn parse_reranker_model(model_str: &str) -> Result<RerankerModel> {
    match model_str.to_lowercase().as_str() {
        "bge-reranker-base" | "baai/bge-reranker-base" => Ok(RerankerModel::BGERerankerBase),
        "bge-reranker-v2-m3" | "baai/bge-reranker-v2-m3" => Ok(RerankerModel::BGERerankerV2M3),
        "jina-reranker-v1-turbo-en" | "jinaai/jina-reranker-v1-turbo-en" => {
            Ok(RerankerModel::JINARerankerV1TurboEn)
        }
        "jina-reranker-v2-base-multilingual" | "jinaai/jina-reranker-v2-base-multilingual" => {
            Ok(RerankerModel::JINARerankerV2BaseMultiligual)
        }
        _ => Err(anyhow::anyhow!("Unknown reranker model: {}", model_str)),
    }
}

/// Get list of available reranker models
pub fn available_models() -> Vec<(&'static str, &'static str)> {
    vec![
        ("bge-reranker-base", "BAAI/bge-reranker-base (default)"),
        ("bge-reranker-v2-m3", "BAAI/bge-reranker-v2-m3"),
        (
            "jina-reranker-v1-turbo-en",
            "jinaai/jina-reranker-v1-turbo-en",
        ),
        (
            "jina-reranker-v2-base-multilingual",
            "jinaai/jina-reranker-v2-base-multilingual",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_reranker_model() {
        let result1 = parse_reranker_model("bge-reranker-base");
        let result2 = parse_reranker_model("BAAI/bge-reranker-base");
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(parse_reranker_model("invalid-model").is_err());
    }

    #[test]
    fn test_rerank_disabled() {
        let config = RerankerConfig {
            enabled: false,
            ..Default::default()
        };

        let reranker = Reranker::new(config).unwrap();

        let docs = vec![RerankDocument {
            content: "Test document".to_string(),
            metadata: HashMap::new(),
        }];

        let result = reranker.rerank("test query", docs.clone(), None).unwrap();
        assert_eq!(result.documents.len(), 1);
        assert_eq!(result.rerank_scores.len(), 0);
    }

    #[test]
    fn test_custom_model_config() {
        let config = RerankerConfig {
            enabled: true,
            custom_model: Some("custom/model".to_string()),
            ..Default::default()
        };

        assert!(config.custom_model.is_some());
        assert_eq!(config.custom_model.as_ref().unwrap(), "custom/model");
    }
}
