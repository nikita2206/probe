mod file_scanner;
mod search_index;
mod metadata;
mod search_engine;
mod config;
mod code_chunker;

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
    
    #[arg(short = 'n', long = "num-results", help = "Number of results to return", default_value = "10")]
    num_results: usize,
    
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
    let engine = SearchEngine::new(&root_dir)?;
    
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
                let results = engine.search(&query, Some(cli.num_results), cli.filetype.as_deref())?;
                
                if results.is_empty() {
                    println!("No results found for '{}'", query);
                } else {
                    println!("Found {} results for '{}':\n", results.len(), query);
                    for (i, result) in results.iter().enumerate() {
                        // Format chunk information
                        let chunk_info = if let (Some(chunk_type), Some(chunk_name)) = (&result.chunk_type, &result.chunk_name) {
                            if !chunk_name.is_empty() {
                                format!(" - {} {}", chunk_type, chunk_name)
                            } else {
                                format!(" - {}", chunk_type)
                            }
                        } else {
                            String::new()
                        };
                        
                        // Format line information
                        let line_info = if let (Some(start), Some(end)) = (result.start_line, result.end_line) {
                            if start == end {
                                format!(" (line {})", start + 1)
                            } else {
                                format!(" (lines {}-{})", start + 1, end + 1)
                            }
                        } else {
                            String::new()
                        };
                        
                        println!("{}. {}{}{} (score: {:.2})", 
                            i + 1, 
                            result.path.display(),
                            chunk_info,
                            line_info,
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
