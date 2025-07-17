use anyhow::Result;
use clap::{Parser, Subcommand};
use fastembed::RerankerModel;
use probe::{available_models, parse_reranker_model, ProbeConfig, RerankerConfig, SearchEngine};
use std::path::PathBuf;

mod show_chunks;

#[derive(Parser)]
#[command(name = "probe")]
#[command(about = "Fast code search with persistent indexing")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, help = "Directory to search")]
    directory: Option<String>,

    #[arg(short = 't', long, help = "Filter by file type (extension)")]
    filetype: Option<String>,

    #[arg(
        short = 'n',
        long = "num-results",
        help = "Number of results to return",
        default_value = "3"
    )]
    num_results: usize,

    #[arg(long = "no-rerank", help = "Disable reranking of search results")]
    no_rerank: bool,

    #[arg(
        long = "rerank-model",
        help = "Reranking model to use (built-in: bge-reranker-base, bge-reranker-v2-m3, etc. or custom model name from config)"
    )]
    rerank_model: Option<String>,

    #[arg(
        long = "rerank-candidates",
        help = "Minimum candidates to fetch for reranking",
        default_value = "10"
    )]
    rerank_candidates: usize,

    #[arg(
        long = "config",
        help = "Path to configuration file (default: ~/.probe/config.yaml)"
    )]
    config_path: Option<PathBuf>,

    #[arg(help = "Search query")]
    query: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Rebuild search index")]
    Rebuild,
    #[command(about = "Show index statistics")]
    Stats,
    #[command(about = "List available reranking models")]
    ListModels,
    #[command(about = "Show how files are chunked for indexing")]
    ShowChunks {
        #[arg(help = "Files or directories to show chunks for (default: current directory)")]
        paths: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root_dir = cli.directory.unwrap_or_else(|| ".".to_string());

    match cli.command {
        Some(Commands::Rebuild) => {
            let engine = SearchEngine::new(&root_dir)?;
            engine.rebuild_index()?;
        }
        Some(Commands::Stats) => {
            let engine = SearchEngine::new(&root_dir)?;
            engine.stats()?;
        }
        Some(Commands::ListModels) => {
            println!("Available reranking models:");
            for (name, description) in available_models() {
                println!("  {name}: {description}");
            }
        }
        Some(Commands::ShowChunks { paths }) => {
            show_chunks::show_chunks_command(paths)?;
        }
        None => {
            if let Some(query) = cli.query {
                // Load configuration
                let probe_config = ProbeConfig::load_from_file(cli.config_path.as_ref())?;

                // Determine which model to use - check if it's a built-in model or custom model
                let (builtin_model, custom_model) = if let Some(model_name) = &cli.rerank_model {
                    if let Ok(builtin) = parse_reranker_model(model_name) {
                        // It's a built-in model
                        (builtin, None)
                    } else if probe_config.get_custom_model(model_name).is_some() {
                        // It's a custom model from config
                        (
                            RerankerModel::JINARerankerV1TurboEn,
                            Some(model_name.clone()),
                        )
                    // Use default built-in as fallback
                    } else {
                        return Err(anyhow::anyhow!("Unknown reranker model '{}'. Use a built-in model (bge-reranker-base, bge-reranker-v2-m3, etc.) or add it to your config file.", model_name));
                    }
                } else if let Some(default_custom) = &probe_config.default_reranker {
                    // Use default custom model from config
                    (
                        RerankerModel::JINARerankerV1TurboEn,
                        Some(default_custom.clone()),
                    )
                } else {
                    // Fall back to built-in default
                    (RerankerModel::JINARerankerV1TurboEn, None)
                };

                // Create reranker config
                let reranker_config = RerankerConfig {
                    enabled: !cli.no_rerank,
                    model: builtin_model,
                    min_candidates: cli.rerank_candidates,
                    show_download_progress: true,
                    custom_model,
                    probe_config: Some(probe_config),
                };

                let engine = SearchEngine::new(&root_dir)?;
                engine.ensure_index_updated()?;
                let results = engine.search_with_reranker(
                    &query,
                    Some(cli.num_results),
                    cli.filetype.as_deref(),
                    reranker_config,
                )?;

                if results.is_empty() {
                    eprintln!("No results found for '{query}'");
                } else {
                    eprintln!("Found {} results for '{}':\n", results.len(), query);
                    for result in results.iter() {
                        // Format line information
                        let line_info = if let (Some(start), Some(end)) =
                            (result.start_line, result.end_line)
                        {
                            if start == end {
                                format!(" (line {})", start + 1)
                            } else {
                                format!(" (lines {}-{})", start + 1, end + 1)
                            }
                        } else {
                            String::new()
                        };

                        println!("{}{}", result.path.display(), line_info);
                        if !result.snippet.is_empty() {
                            println!("{}\n", result.snippet);
                        }
                    }
                }
            } else {
                println!("Usage: probe <query> or probe --help");
            }
        }
    }

    Ok(())
}
