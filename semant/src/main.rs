use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <file.cl>", args[0]);
        return ExitCode::from(1);
    }
    
    let filename = &args[1];
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", filename, e);
            return ExitCode::from(1);
        }
    };
    
    // Parse
    let ast = match parser::parse_source(&source) {
        Ok(ast) => ast,
        Err(errors) => {
            eprintln!("Parse errors in {}:", filename);
            for e in errors {
                eprintln!("  {}", e);
            }
            return ExitCode::from(1);
        }
    };
    
    // Semantic analysis
    let mut analyzer = semant::SemanticAnalyzer::new();
    match analyzer.analyze(&ast) {
        Ok(()) => {
            println!("Semantic analysis passed: {}", filename);
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("Semantic errors in {}:", filename);
            for e in &errors {
                eprintln!("  {:?}", e);
            }
            ExitCode::from(1)
        }
    }
}
