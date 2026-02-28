//! COOL to WebAssembly Compiler CLI
//!
//! Command-line interface for compiling COOL programs to WebAssembly.

use codegen::compile_hir;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = if args.len() >= 4 && args[2] == "-o" {
        args[3].clone()
    } else {
        // Default output: replace .cl with .wasm
        input_path.replace(".cl", ".wasm")
    };

    // Read input file
    let source = match std::fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading input file: {}", e);
            std::process::exit(1);
        }
    };

    println!("Compiling {} to {}", input_path, output_path);

    // Full compilation pipeline: Parser → Semant → AST-to-HIR → Codegen
    
    // 1. Parsing (includes lexing)
    let ast = match parser::parse_source(&source) {
        Ok(ast) => ast,
        Err(errors) => {
            eprintln!("Parse errors:");
            for error in errors {
                eprintln!("  {}", error);
            }
            std::process::exit(1);
        }
    };

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
    eprintln!("Usage: {} <input.cl> [-o <output.wasm>]", program_name);
    eprintln!();
    eprintln!("Compile a COOL program to WebAssembly.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>    Specify output file (default: input with .wasm extension)");
}
