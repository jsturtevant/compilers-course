//! COOL to WebAssembly Compiler CLI
//!
//! Command-line interface for compiling COOL programs to WebAssembly.
//! Supports multi-file compilation by merging classes from multiple .cl files.

use codegen::compile_hir;
use parser::Program;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    // Parse arguments: collect input files and output path
    let mut input_files: Vec<String> = Vec::new();
    let mut output_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "-o" {
            if i + 1 < args.len() {
                output_path = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!("Error: -o requires an output path");
                std::process::exit(1);
            }
        } else if args[i].ends_with(".cl") {
            input_files.push(args[i].clone());
            i += 1;
        } else {
            eprintln!("Warning: ignoring unknown argument: {}", args[i]);
            i += 1;
        }
    }

    if input_files.is_empty() {
        eprintln!("Error: no input files specified");
        print_usage(&args[0]);
        std::process::exit(1);
    }

    // Default output: based on first input file
    let output_path = output_path.unwrap_or_else(|| {
        input_files[0].replace(".cl", ".wasm")
    });

    println!("Compiling {} file(s) to {}", input_files.len(), output_path);

    // Full compilation pipeline: Parser → Semant → AST-to-HIR → Codegen
    
    // 1. Parsing (includes lexing) - parse all files and merge classes
    let mut all_classes = Vec::new();
    for input_path in &input_files {
        let source = match std::fs::read_to_string(input_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {}: {}", input_path, e);
                std::process::exit(1);
            }
        };

        match parser::parse_source(&source) {
            Ok(ast) => {
                println!("  Parsed {} ({} classes)", input_path, ast.classes.len());
                all_classes.extend(ast.classes);
            }
            Err(errors) => {
                eprintln!("Parse errors in {}:", input_path);
                for error in errors {
                    eprintln!("  {}", error);
                }
                std::process::exit(1);
            }
        }
    }

    // Merge all classes into a single program
    let ast = Program { classes: all_classes };
    println!("  Total: {} classes", ast.classes.len());

    // 2. Semantic analysis
    let mut analyzer = semant::SemanticAnalyzer::new();
    if let Err(errors) = analyzer.analyze(&ast) {
        eprintln!("Semantic errors:");
        for error in errors {
            eprintln!("  {:?}", error);
        }
        std::process::exit(1);
    }

    // 3. AST to HIR
    let hir = match ir::ast_to_hir::lower_program(&ast, &analyzer) {
        Ok(hir) => hir,
        Err(e) => {
            eprintln!("HIR lowering error: {}", e);
            std::process::exit(1);
        }
    };

    // 4. Compile HIR to WASM
    match compile_hir(&hir) {
        Ok(wasm_bytes) => {
            // Write output
            if let Err(e) = std::fs::write(&output_path, wasm_bytes) {
                eprintln!("Error writing output file: {}", e);
                std::process::exit(1);
            }
            println!("Successfully compiled to {}", output_path);
        }
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    eprintln!("Usage: {} <input1.cl> [input2.cl ...] [-o <output.wasm>]", program_name);
    eprintln!();
    eprintln!("Compile one or more COOL files to WebAssembly.");
    eprintln!("Classes from all input files are merged into a single program.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>    Specify output file (default: first input with .wasm extension)");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} hello.cl -o hello.wasm", program_name);
    eprintln!("  {} atoi.cl atoi_test.cl -o atoi_test.wasm", program_name);
}
