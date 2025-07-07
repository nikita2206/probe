use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::tokenizer::Language;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub stemming: StemmingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemmingConfig {
    pub language: String,
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            stemming: StemmingConfig {
                language: "english".to_string(),
                enabled: true,
            },
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let content = std::fs::read_to_string(config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_from_dir<P: AsRef<Path>>(dir_path: P) -> Result<Self> {
        let config_path = dir_path.as_ref().join("probe.yml");
        if config_path.exists() {
            Self::load_from_file(config_path)
        } else {
            Ok(Config::default())
        }
    }

    pub fn get_language(&self) -> Result<Language> {
        if !self.stemming.enabled {
            return Ok(Language::English); // Default fallback, but stemming will be disabled
        }

        match self.stemming.language.to_lowercase().as_str() {
            "english" | "en" => Ok(Language::English),
            "french" | "fr" => Ok(Language::French),
            "german" | "de" => Ok(Language::German),
            "italian" | "it" => Ok(Language::Italian),
            "portuguese" | "pt" => Ok(Language::Portuguese),
            "spanish" | "es" => Ok(Language::Spanish),
            "dutch" | "nl" => Ok(Language::Dutch),
            "danish" | "da" => Ok(Language::Danish),
            "finnish" | "fi" => Ok(Language::Finnish),
            "hungarian" | "hu" => Ok(Language::Hungarian),
            "norwegian" | "no" => Ok(Language::Norwegian),
            "romanian" | "ro" => Ok(Language::Romanian),
            "russian" | "ru" => Ok(Language::Russian),
            "swedish" | "sv" => Ok(Language::Swedish),
            "tamil" | "ta" => Ok(Language::Tamil),
            "turkish" | "tr" => Ok(Language::Turkish),
            other => Err(anyhow::anyhow!("Unsupported language: {}", other)),
        }
    }
}
