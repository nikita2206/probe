use anyhow::{Context, Result};
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the reranker
#[derive(Debug, Clone)]
pub struct RerankerConfig {
    pub enabled: bool,
    pub model: RerankerModel,
    pub min_candidates: usize,
    pub show_download_progress: bool,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: RerankerModel::BGERerankerBase,
            min_candidates: 10,
            show_download_progress: false,
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

        let model = TextRerank::try_new(
            RerankInitOptions::new(config.model.clone())
                .with_show_download_progress(config.show_download_progress)
                .with_cache_dir(cache_dir),
        )
        .context("Failed to initialize reranking model")?;

        Ok(Self {
            model: Arc::new(model),
            config,
        })
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
}
