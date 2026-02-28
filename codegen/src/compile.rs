//! End-to-end compilation pipeline
//!
//! This module provides the complete compilation pipeline from COOL source code
//! to WebAssembly binary output.

use crate::emit;
use ir::hir::HirProgram;
use ir::lower::LoweringContext;

#[derive(Debug)]
pub enum CompileError {
    /// Error during HIR → LIR lowering
    LoweringError(String),
    /// Error during LIR → WASM emission
    EmissionError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::LoweringError(msg) => write!(f, "Lowering error: {}", msg),
            CompileError::EmissionError(msg) => write!(f, "Emission error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile HIR to WASM binary
///
/// This function performs the complete backend pipeline:
/// 1. HIR → LIR lowering
/// 2. LIR → WASM emission
///
/// Returns a WebAssembly binary ready for execution.
pub fn compile_hir(hir: &HirProgram) -> Result<Vec<u8>, CompileError> {
    // Lower HIR to LIR
    let mut ctx = LoweringContext::new();
    let lir = ctx.lower_program(hir);

    // Emit WASM from LIR (pass HIR for _start generation)
    let wasm = emit::emit_module(&lir, hir)
        .map_err(CompileError::EmissionError)?;

    Ok(wasm)
}

/// Compile HIR to WASM and write to file
pub fn compile_to_file(hir: &HirProgram, output_path: &str) -> Result<(), CompileError> {
    let wasm = compile_hir(hir)?;
    
    std::fs::write(output_path, wasm)
        .map_err(|e| CompileError::EmissionError(format!("Failed to write output: {}", e)))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::hir::*;

    #[test]
    fn test_compile_simple_program() {
        // Create a minimal HIR program with one class and one method
        let program = HirProgram {
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
                            body: HirExpr::IntLiteral {
                                value: 42,
                                typ: TypeId::Int,
                            },
                            vtable_index: 0,
                        }
                    ],
                }
            ],
        };

        let result = compile_hir(&program);
        assert!(result.is_ok());
        
        let wasm = result.unwrap();
        // Check WASM magic number
        assert_eq!(&wasm[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        // Check version
        assert_eq!(&wasm[4..8], &[0x01, 0x00, 0x00, 0x00]);
    }
}
