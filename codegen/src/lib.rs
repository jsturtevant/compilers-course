//! COOL to WebAssembly Code Generator
//!
//! This crate provides the backend for the COOL compiler, generating
//! WebAssembly modules from the intermediate representation.
//!
//! # Architecture
//!
//! The code generator is organized into several modules:
//!
//! - `wasm`: WASM module builder and section management
//! - `runtime`: COOL runtime support in WASM (Object, IO, String builtins)
//! - `emit`: IR to WASM instruction emission
//! - `compile`: End-to-end compilation pipeline
//!
//! # Usage
//!
//! ```ignore
//! use codegen::compile::compile_hir;
//!
//! let wasm_bytes = compile_hir(&hir_program)?;
//! std::fs::write("output.wasm", wasm_bytes)?;
//! ```

pub mod compile;
pub mod emit;
pub mod runtime;
pub mod wasm;

pub use compile::{compile_hir, compile_to_file, CompileError};
pub use wasm::WasmModule;
