//! COOL runtime support in WebAssembly
//!
//! This module provides the runtime functions required by COOL programs:
//! - Object methods (abort, type_name, copy)
//! - IO methods (out_string, out_int, in_string, in_int)
//! - String methods (length, concat, substr)
//!
//! For MVP, IO is implemented using WASI (fd_read/fd_write).

use crate::wasm::WasmModule;
use wasm_encoder::{Function, Instruction, ValType};

/// Runtime function IDs
///
/// These indices are used to reference runtime functions in generated code.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeFunctions {
    pub object_abort: u32,
    pub object_type_name: u32,
    pub object_copy: u32,
    pub io_out_string: u32,
    pub io_out_int: u32,
    pub io_in_string: u32,
    pub io_in_int: u32,
    pub string_length: u32,
    pub string_concat: u32,
    pub string_substr: u32,
    pub wasi_fd_write: u32,
    pub wasi_fd_read: u32,
}

/// Add COOL runtime support to a WASM module
///
/// This function:
/// 1. Imports WASI functions (fd_read, fd_write)
/// 2. Defines runtime functions for Object, IO, and String
/// 3. Returns function indices for use in codegen
pub fn add_runtime(module: &mut WasmModule) -> RuntimeFunctions {
    // WASI imports
    // fd_write(fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> i32
    let fd_write_type = module.add_type(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    module.import_wasi("fd_write", fd_write_type);
    let wasi_fd_write = 0; // First import

    // fd_read(fd: i32, iovs: i32, iovs_len: i32, nread: i32) -> i32
    let fd_read_type = module.add_type(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    module.import_wasi("fd_read", fd_read_type);
    let wasi_fd_read = 1; // Second import

    // Object methods
    let object_abort = add_object_abort(module);
    let object_type_name = add_object_type_name(module);
    let object_copy = add_object_copy(module);

    // IO methods
    let io_out_string = add_io_out_string(module, wasi_fd_write);
    let io_out_int = add_io_out_int(module, wasi_fd_write);
    let io_in_string = add_io_in_string(module, wasi_fd_read);
    let io_in_int = add_io_in_int(module, wasi_fd_read);

    // String methods
    let string_length = add_string_length(module);
    let string_concat = add_string_concat(module);
    let string_substr = add_string_substr(module);

    RuntimeFunctions {
        object_abort,
        object_type_name,
        object_copy,
        io_out_string,
        io_out_int,
        io_in_string,
        io_in_int,
        string_length,
        string_concat,
        string_substr,
        wasi_fd_write,
        wasi_fd_read,
    }
}

// Object methods

/// Object.abort() -> noreturn
///
/// Prints an error message and terminates execution.
/// For MVP: just unreachable (trap)
fn add_object_abort(module: &mut WasmModule) -> u32 {
    let type_idx = module.add_type(vec![ValType::I32], vec![]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Print abort message using WASI fd_write
    func.instruction(&Instruction::Unreachable);
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// Object.type_name() -> String
///
/// Returns the class name as a String object.
/// Object pointer at offset 0 contains class_tag.
fn add_object_type_name(module: &mut WasmModule) -> u32 {
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Load class_tag from object, lookup in class metadata table
    // For now: return null (0)
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// Object.copy() -> Object
///
/// Creates a shallow copy of the object.
fn add_object_copy(module: &mut WasmModule) -> u32 {
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Allocate new object, copy bytes from source
    // For now: return the same object (identity)
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

// IO methods

/// IO.out_string(s: String) -> IO
///
/// Writes a string to stdout using WASI fd_write.
fn add_io_out_string(module: &mut WasmModule, _wasi_fd_write: u32) -> u32 {
    // (self: i32, s: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Extract string data, set up iovec, call fd_write
    // For now: return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.out_int(i: Int) -> IO
///
/// Converts an integer to string and writes to stdout.
fn add_io_out_int(module: &mut WasmModule, _wasi_fd_write: u32) -> u32 {
    // (self: i32, i: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Convert int to string, call out_string
    // For now: return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.in_string() -> String
///
/// Reads a string from stdin using WASI fd_read.
fn add_io_in_string(module: &mut WasmModule, _wasi_fd_read: u32) -> u32 {
    // (self: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Call fd_read, allocate String object
    // For now: return empty string (null)
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.in_int() -> Int
///
/// Reads a string from stdin and parses it as an integer.
fn add_io_in_int(module: &mut WasmModule, _wasi_fd_read: u32) -> u32 {
    // (self: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Call in_string, parse to int
    // For now: return 0
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

// String methods

/// String.length() -> Int
///
/// Returns the length of the string.
/// String object layout: class_tag (i32) + size (i32) + vtable_ptr (i32) + length (i32) at offset 12
fn add_string_length(module: &mut WasmModule) -> u32 {
    // (self: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // Load length from offset 12
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Const(12));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// String.concat(s: String) -> String
///
/// Concatenates two strings.
fn add_string_concat(module: &mut WasmModule) -> u32 {
    // (self: i32, s: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Allocate new string, copy both string data
    // For now: return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// String.substr(i: Int, l: Int) -> String
///
/// Returns a substring starting at position i with length l.
fn add_string_substr(module: &mut WasmModule) -> u32 {
    // (self: i32, i: i32, l: i32) -> i32
    let type_idx = module.add_type(
        vec![ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    // TODO: Validate bounds, allocate new string, copy substring
    // For now: return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}
