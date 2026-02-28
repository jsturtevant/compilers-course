//! COOL to WebAssembly Compiler CLI
//!
//! Command-line interface for compiling COOL programs to WebAssembly.

use codegen::compile_hir;
use ir::hir::{HirProgram, HirClass, HirMethod, HirExpr, TypeId};

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

    // Read input file (for validation, but not used yet)
    let _source = match std::fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading input file: {}", e);
            std::process::exit(1);
        }
    };

    println!("Compiling {} to {}", input_path, output_path);

    // TODO: Integrate with lexer, parser, semant, and ast_to_hir
    // For now, create a stub HIR program for testing
    let hir = create_test_hir();

    // Compile HIR to WASM
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

/// Create a test HIR program for demonstration
fn create_test_hir() -> HirProgram {
    HirProgram {
        classes: vec![
            HirClass {
                name: "Main".to_string(),
                parent: None,
                class_tag: 0,
                attributes: vec![],
                methods: vec![
                    HirMethod {
                        name: "main".to_string(),
                        formals: vec![],
                        return_type: TypeId::Int,
                        body: HirExpr::Add {
                            left: Box::new(HirExpr::IntLiteral {
                                value: 40,
                                typ: TypeId::Int,
                            }),
                            right: Box::new(HirExpr::IntLiteral {
                                value: 2,
                                typ: TypeId::Int,
                            }),
                            typ: TypeId::Int,
                        },
                        vtable_index: 0,
                    }
                ],
            }
        ],
    }
}
