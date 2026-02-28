//! COOL to WebAssembly Compiler CLI
//!
//! Command-line interface for compiling COOL programs to WebAssembly.
//! Supports multi-file compilation by merging classes from multiple .cl files.
//! Can output either core WASM modules or WASM components.

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
    let mut emit_component = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: -o requires an output path");
                    std::process::exit(1);
                }
            }
            "--component" | "-c" => {
                emit_component = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage(&args[0]);
                std::process::exit(0);
            }
            arg if arg.ends_with(".cl") => {
                input_files.push(args[i].clone());
                i += 1;
            }
            _ => {
                eprintln!("Warning: ignoring unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    if input_files.is_empty() {
        eprintln!("Error: no input files specified");
        print_usage(&args[0]);
        std::process::exit(1);
    }

    // Default output: based on first input file
    let output_path = output_path.unwrap_or_else(|| input_files[0].replace(".cl", ".wasm"));

    let output_type = if emit_component {
        "component"
    } else {
        "module"
    };
    println!(
        "Compiling {} file(s) to {} ({})",
        input_files.len(),
        output_path,
        output_type
    );

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
    let ast = Program {
        classes: all_classes,
    };
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
            // Optionally wrap as component
            let final_bytes = if emit_component {
                match encode_component(&wasm_bytes) {
                    Ok(component_bytes) => {
                        println!(
                            "  Wrapped as WASM component ({} bytes)",
                            component_bytes.len()
                        );
                        component_bytes
                    }
                    Err(e) => {
                        eprintln!("Component encoding error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                wasm_bytes
            };

            // Write output
            if let Err(e) = std::fs::write(&output_path, final_bytes) {
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

/// Encode a core WASM module as a WASM component
fn encode_component(core_wasm: &[u8]) -> Result<Vec<u8>, String> {
    use wasi_preview1_component_adapter_provider::WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER;
    use wit_component::ComponentEncoder;

    // Create a component that wraps the core module with WASI adapter
    let encoded = ComponentEncoder::default()
        .module(core_wasm)
        .map_err(|e| format!("Failed to set module: {}", e))?
        .adapter(
            "wasi_snapshot_preview1",
            WASI_SNAPSHOT_PREVIEW1_COMMAND_ADAPTER,
        )
        .map_err(|e| format!("Failed to add WASI adapter: {}", e))?
        .validate(true)
        .encode()
        .map_err(|e| format!("Failed to encode component: {}", e))?;

    Ok(encoded)
}

fn print_usage(program_name: &str) {
    eprintln!(
        "Usage: {} <input1.cl> [input2.cl ...] [-o <output.wasm>] [--component]",
        program_name
    );
    eprintln!();
    eprintln!("Compile one or more COOL files to WebAssembly.");
    eprintln!("Classes from all input files are merged into a single program.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -o <output>     Specify output file (default: first input with .wasm extension)");
    eprintln!("  -c, --component Output a WASM component instead of a core module");
    eprintln!("  -h, --help      Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} hello.cl -o hello.wasm", program_name);
    eprintln!("  {} hello.cl --component -o hello.wasm", program_name);
    eprintln!("  {} atoi.cl atoi_test.cl -o atoi_test.wasm", program_name);
}
