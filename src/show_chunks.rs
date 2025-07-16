use anyhow::Result;
use std::fs;
use std::path::Path;

use probe::{CodeChunker, FileScanner, ChunkType};

pub fn show_chunks_command(paths: Vec<String>) -> Result<()> {
    let mut chunker = CodeChunker::new()?;
    
    if paths.is_empty() {
        // No paths provided, scan current directory
        show_chunks_for_directory(Path::new("."), &mut chunker)?;
    } else {
        for path_str in paths {
            let path = Path::new(&path_str);
            
            if path.is_file() {
                show_chunks_for_file(path, &mut chunker)?;
            } else if path.is_dir() {
                show_chunks_for_directory(path, &mut chunker)?;
            } else {
                eprintln!("Warning: '{}' is not a valid file or directory", path_str);
            }
        }
    }
    
    Ok(())
}

fn show_chunks_for_file(file_path: &Path, chunker: &mut CodeChunker) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let chunks = chunker.chunk_code_for_indexing(file_path, &content)?;
    
    for chunk in chunks {
        println!("{}:{}-{} [{}] {}", 
            file_path.display(),
            chunk.start_line + 1,
            chunk.end_line + 1,
            format_chunk_type(&chunk.chunk_type),
            chunk.name
        );
        
        // Print the declaration if it exists
        if !chunk.declaration.is_empty() {
            println!("Declaration:");
            for line in chunk.declaration.lines() {
                println!("  {}", line);
            }
        }
        
        // Print the content with line numbers
        println!("Content:");
        for (i, line) in chunk.content.lines().enumerate() {
            println!("  {:4}: {}", chunk.start_line + i + 1, line);
        }
        
        println!(); // Empty line between chunks
    }
    
    Ok(())
}

fn show_chunks_for_directory(dir_path: &Path, chunker: &mut CodeChunker) -> Result<()> {
    let scanner = FileScanner::new(dir_path);
    
    for file_path in scanner.iter_files() {
        match show_chunks_for_file(file_path.as_path(), chunker) {
            Ok(()) => {},
            Err(e) => {
                eprintln!("Error processing file '{}': {}", file_path.display(), e);
            }
        }
    }
    
    Ok(())
}

fn format_chunk_type(chunk_type: &ChunkType) -> &'static str {
    match chunk_type {
        ChunkType::Function => "function",
        ChunkType::Method => "method",
        ChunkType::Class => "class",
        ChunkType::Struct => "struct",
        ChunkType::Interface => "interface",
        ChunkType::Module => "module",
        ChunkType::Other => "other",
    }
}
