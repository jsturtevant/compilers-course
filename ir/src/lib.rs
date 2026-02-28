pub mod ast_to_hir;
/// IR crate for transforming typed AST into intermediate representations for WASM codegen
///
/// This crate implements a two-tier IR (HIR → LIR) for the COOL compiler:
/// - HIR (High-level IR): Typed AST with resolved types and method dispatch info
/// - LIR (Low-level IR): Linear WASM-like instructions ready for codegen
pub mod hir;
pub mod lir;
pub mod lower;

pub use ast_to_hir::lower_program;
pub use hir::*;
pub use lir::*;
pub use lower::*;
