mod file_scanner;
mod search_index;
mod metadata;
mod search_engine;

use clap::{Parser, Subcommand};
use anyhow::Result;
use search_engine::SearchEngine;

#[derive(Parser)]
#[command(name = "codesearch")]
#[command(about = "Fast code search with persistent indexing")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(short, long, help = "Directory to search")]
    directory: Option<String>,
    
    #[arg(short = 't', long, help = "Filter by file type (extension)")]
    filetype: Option<String>,
    
    #[arg(help = "Search query")]
    query: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Rebuild search index")]
    Rebuild,
    #[command(about = "Show index statistics")]
    Stats,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let root_dir = cli.directory.unwrap_or_else(|| ".".to_string());
    let engine = SearchEngine::new(&root_dir);
    
    match cli.command {
        Some(Commands::Rebuild) => {
            engine.rebuild_index()?;
        }
        Some(Commands::Stats) => {
            engine.stats()?;
        }
        None => {
            if let Some(query) = cli.query {
                engine.ensure_index_updated()?;
                let results = engine.search(&query, None, cli.filetype.as_deref())?;
                
                if results.is_empty() {
                    println!("No results found for '{}'", query);
                } else {
                    println!("Found {} results for '{}':\n", results.len(), query);
                    for (i, result) in results.iter().enumerate() {
                        println!("{}. {} (score: {:.2})", 
                            i + 1, 
                            result.path.display(), 
                            result.score
                        );
                        if !result.snippet.is_empty() {
                            println!("   {}\n", result.snippet);
                        }
                    }
                }
            } else {
                println!("Usage: codesearch <query> or codesearch --help");
            }
        }
    }
    
    Ok(())
}
