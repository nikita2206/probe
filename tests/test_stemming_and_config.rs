use probe::config::Config;
use probe::reranker::RerankerConfig;
use probe::search_engine::SearchEngine;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_stemming_functionality() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with plural words
    let test_file = temp_dir.path().join("test.java");
    fs::write(
        &test_file,
        "public void handleCarriers() { List<String> runners = new ArrayList<>(); }",
    )
    .unwrap();

    // Create config with stemming enabled
    let config_content = r#"
stemming:
  enabled: true
  language: english
"#;
    let config_file = temp_dir.path().join("probe.yml");
    fs::write(&config_file, config_content).unwrap();

    // Create search engine and rebuild index
    let engine = SearchEngine::new(temp_dir.path()).unwrap();
    engine.rebuild_index().unwrap();

    // Test that singular form matches plural
    let reranker_config = RerankerConfig {
        enabled: false,
        ..Default::default()
    };
    let results = engine
        .search_with_reranker("carrier", Some(10), None, reranker_config.clone())
        .unwrap();
    assert!(
        !results.is_empty(),
        "Should find 'carriers' when searching for 'carrier'"
    );
    assert!(results[0].path.ends_with("test.java"));

    // Test that singular form matches plural for 'runner'
    let results = engine
        .search_with_reranker("runner", Some(10), None, reranker_config)
        .unwrap();
    assert!(
        !results.is_empty(),
        "Should find 'runners' when searching for 'runner'"
    );
    assert!(results[0].path.ends_with("test.java"));

    // Clean up temp files
    fs::remove_file(&test_file).unwrap();
    fs::remove_file(&config_file).unwrap();
}

#[test]
fn test_stemming_disabled() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with plural words
    let test_file = temp_dir.path().join("test.java");
    fs::write(
        &test_file,
        "public void handleCarriers() { List<String> runners = new ArrayList<>(); }",
    )
    .unwrap();

    // Create config with stemming disabled
    let config_content = r#"
stemming:
  enabled: false
  language: english
"#;
    let config_file = temp_dir.path().join("probe.yml");
    fs::write(&config_file, config_content).unwrap();

    // Create search engine and rebuild index
    let engine = SearchEngine::new(temp_dir.path()).unwrap();
    engine.rebuild_index().unwrap();

    // Test that singular form does NOT match plural when stemming is disabled
    let reranker_config = RerankerConfig {
        enabled: false,
        ..Default::default()
    };
    let results = engine
        .search_with_reranker("carrier", Some(10), None, reranker_config.clone())
        .unwrap();
    assert!(
        results.is_empty(),
        "Should not find 'carriers' when searching for 'carrier' with stemming disabled"
    );

    // But exact matches should still work
    let results = engine
        .search_with_reranker("carriers", Some(10), None, reranker_config)
        .unwrap();
    assert!(!results.is_empty(), "Should find exact match 'carriers'");

    // Clean up temp files
    fs::remove_file(&test_file).unwrap();
    fs::remove_file(&config_file).unwrap();
}

#[test]
fn test_config_loading() {
    let temp_dir = TempDir::new().unwrap();

    // Test default config (no file)
    let config = Config::load_from_dir(temp_dir.path()).unwrap();
    assert!(config.stemming.enabled);
    assert_eq!(config.stemming.language, "english");

    // Test custom config
    let config_content = r#"
stemming:
  enabled: false
  language: french
"#;
    let config_file = temp_dir.path().join("probe.yml");
    fs::write(&config_file, config_content).unwrap();

    let config = Config::load_from_dir(temp_dir.path()).unwrap();
    assert!(!config.stemming.enabled);
    assert_eq!(config.stemming.language, "french");

    // Clean up temp file
    fs::remove_file(&config_file).unwrap();
}

#[test]
fn test_different_languages() {
    let temp_dir = TempDir::new().unwrap();

    // Test different language configurations
    let languages = vec![
        ("english", "en"),
        ("french", "fr"),
        ("german", "de"),
        ("spanish", "es"),
    ];

    for (lang_full, lang_short) in languages {
        // Test full language name
        let config_content = format!(
            r#"
stemming:
  enabled: true
  language: {lang_full}
"#,
        );
        let config_file = temp_dir.path().join("probe.yml");
        fs::write(&config_file, &config_content).unwrap();

        let config = Config::load_from_dir(temp_dir.path()).unwrap();
        assert!(
            config.get_language().is_ok(),
            "Should support language: {lang_full}"
        );

        // Test short language code
        let config_content = format!(
            r#"
stemming:
  enabled: true
  language: {lang_short}
"#,
        );
        fs::write(&config_file, &config_content).unwrap();

        let config = Config::load_from_dir(temp_dir.path()).unwrap();
        assert!(
            config.get_language().is_ok(),
            "Should support language code: {lang_short}"
        );

        fs::remove_file(&config_file).unwrap();
    }
}

#[test]
fn test_invalid_language() {
    let temp_dir = TempDir::new().unwrap();

    let config_content = r#"
stemming:
  enabled: true
  language: invalid_language
"#;
    let config_file = temp_dir.path().join("probe.yml");
    fs::write(&config_file, config_content).unwrap();

    let config = Config::load_from_dir(temp_dir.path()).unwrap();
    assert!(
        config.get_language().is_err(),
        "Should reject invalid language"
    );

    // Clean up temp file
    fs::remove_file(&config_file).unwrap();
}
