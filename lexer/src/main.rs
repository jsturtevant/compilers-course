mod boolean_tests;
mod comments_tests;
mod lexer;
mod string_tests;

use clap::Parser;
use lexer::Token;
use logos::Logos;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "lexer")]
#[command(about = "A lexer for Cool programming language")]
#[command(version)]
struct Args {
    /// Input Cool (.cl) file to lex
    #[arg(value_name = "FILE|DIRECTORY")]
    file: String,

    /// Print verbose output including all tokens
    #[arg(short, long)]
    verbose: bool,
}

fn lex_file(file_path: &Path, verbose: bool) {
    if verbose {
        println!("Lexing file: {}", file_path.display());
    }
    let input = fs::read_to_string(file_path).expect("Failed to read file");

    let mut lexer = Token::lexer(&input);
    let mut success = true;

    while let Some(token) = lexer.next() {
        let span = lexer.span();
        match token {
            Ok(t) => {
                if verbose {
                    println!(
                        "Token: {:?}, Span: {:?}, Text: '{}'",
                        t,
                        span,
                        &input[span.clone()]
                    );
                }
            }
            Err(_) => {
                success = false;

                // Get line and line start from lexer extras
                let line_num = lexer.extras.0;
                let line_start = lexer.extras.1;

                let line_text = input.lines().nth(line_num).unwrap_or("");
                let col_start = span.start - line_start;
                let col_end = span.end - line_start;

                // Calculate visual column position by expanding tabs
                let mut visual_col = 0;
                for (i, ch) in line_text.char_indices() {
                    if i >= col_start {
                        break;
                    }
                    if ch == '\t' {
                        visual_col += 8 - (visual_col % 8); // Tab stops every 8 characters
                    } else {
                        visual_col += 1;
                    }
                }

                let indicator = " ".repeat(visual_col) + &"^".repeat((col_end - col_start).max(1));

                eprintln!(
                    "Error lexing '{}': '{}' at {:?} on line {} column {}\n\n{}\n{}\n",
                    file_path.display(),
                    &input[span.clone()],
                    span,
                    line_num + 1,  // 1-indexed for display
                    col_start + 1, // 1-indexed for display
                    line_text,
                    indicator
                );
            }
        }
    }

    if verbose {
        if success {
            println!("✓ Successfully lexed '{}'", file_path.display());

        }else {
            println!("✗ Failed to lex '{}'", file_path.display());
        }
    }
}

fn main() {
    let args = Args::parse();
    let path = Path::new(&args.file);

    if !path.exists() {
        eprintln!("Error: file or directory '{}' not found", args.file);
        return;
    }

    if path.is_dir() {
        if args.verbose {
            println!("Lexing all .cl files in directory: {}", args.file);
        }
        for entry in fs::read_dir(path).expect("Failed to read directory") {
            let entry = entry.expect("Failed to read directory entry");
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "cl" {
                        lex_file(&file_path, args.verbose);
                    }
                }
            }
        }
    } else {
        lex_file(path, args.verbose);
    }
}
