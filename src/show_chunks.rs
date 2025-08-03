use anyhow::Result;
use std::fs;
use std::path::Path;

use probe::{CodeChunker, FileScanner};

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
                eprintln!("Warning: '{path_str}' is not a valid file or directory");
            }
        }
    }

    Ok(())
}

fn show_chunks_for_file(file_path: &Path, chunker: &mut CodeChunker) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let chunks = chunker.chunk_code_for_indexing(file_path, &content)?;

    if !chunks.is_empty() {
        println!("{}", file_path.display());

        for (i, chunk) in chunks.iter().enumerate() {
            if !chunk.declaration.is_empty() {
                print!("{}", chunk.declaration);
            }
            print!("{}", chunk.content);

            if i < chunks.len() - 1 {
                println!();
                println!("-----");
            }
        }

        println!();
    }

    Ok(())
}

fn show_chunks_for_directory(dir_path: &Path, chunker: &mut CodeChunker) -> Result<()> {
    let scanner = FileScanner::new(dir_path);

    for file_path in scanner.iter_files() {
        match show_chunks_for_file(file_path.as_path(), chunker) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error processing file '{}': {}", file_path.display(), e);
            }
        }
    }

    Ok(())
}
