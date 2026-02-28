//! COOL runtime support in WebAssembly
//!
//! This module provides the runtime functions required by COOL programs:
//! - Memory allocator (bump allocator)
//! - Object methods (abort, type_name, copy)
//! - IO methods (out_string, out_int, in_string, in_int)
//! - String methods (length, concat, substr)
//!
//! For MVP, IO is implemented using WASI (fd_read/fd_write).

use crate::wasm::WasmModule;
use wasm_encoder::{Function, Instruction, MemArg, ValType};

/// Runtime function IDs
///
/// These indices are used to reference runtime functions in generated code.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeFunctions {
    pub alloc: u32,
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
    pub string_equals: u32,
    pub wasi_fd_write: u32,
    pub wasi_fd_read: u32,
}

/// Global indices
const HEAP_PTR_GLOBAL: u32 = 0;

/// Add COOL runtime support to a WASM module
///
/// This function:
/// 1. Imports WASI functions (fd_read, fd_write)
/// 2. Adds heap pointer global
/// 3. Defines runtime functions for allocator, Object, IO, and String
/// 4. Returns function indices for use in codegen
pub fn add_runtime(module: &mut WasmModule, heap_start: u32) -> RuntimeFunctions {
    // Add heap pointer global (starts after static data)
    module.add_global(ValType::I32, true, heap_start as i32);

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

    // Allocator
    let alloc = add_alloc(module);

    // Object methods
    let object_abort = add_object_abort(module);
    let object_type_name = add_object_type_name(module);
    let object_copy = add_object_copy(module, alloc);

    // IO methods
    let io_out_string = add_io_out_string(module, wasi_fd_write);
    let io_out_int = add_io_out_int(module, wasi_fd_write);
    let io_in_string = add_io_in_string(module, wasi_fd_read);
    let io_in_int = add_io_in_int(module, wasi_fd_read);

    // String methods
    let string_length = add_string_length(module);
    let string_concat = add_string_concat(module, alloc);
    let string_substr = add_string_substr(module, alloc);
    let string_equals = add_string_equals(module);

    RuntimeFunctions {
        alloc,
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
        string_equals,
        wasi_fd_write,
        wasi_fd_read,
    }
}

// Memory allocator

/// alloc(size: i32) -> i32
///
/// Bump allocator: allocates memory from the heap.
/// Memory is 4-byte aligned.
fn add_alloc(module: &mut WasmModule) -> u32 {
    // (size: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([(1, ValType::I32)]); // local for return value
    
    // Get current heap pointer and save it
    func.instruction(&Instruction::GlobalGet(HEAP_PTR_GLOBAL));
    func.instruction(&Instruction::LocalSet(1)); // Save to local 1
    
    // Calculate aligned size: (size + 3) & ~3
    func.instruction(&Instruction::LocalGet(0)); // size
    func.instruction(&Instruction::I32Const(3));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Const(-4)); // ~3 = -4 in two's complement
    func.instruction(&Instruction::I32And);
    
    // Add to current heap pointer to get new heap pointer
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32Add);
    
    // Store as new heap pointer
    func.instruction(&Instruction::GlobalSet(HEAP_PTR_GLOBAL));
    
    // Return original pointer
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

// Object methods

/// Object.abort() -> noreturn
///
/// Terminates execution immediately.
/// Returns i32 for WASM type consistency, though it never actually returns.
fn add_object_abort(module: &mut WasmModule) -> u32 {
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    func.instruction(&Instruction::Unreachable);
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// Object.type_name() -> String
///
/// Returns the class name as a String object.
/// Object pointer at offset 0 contains class_tag.
/// TODO: Implement class name table lookup.
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
/// Reads size from object header, allocates new object, and copies bytes.
fn add_object_copy(module: &mut WasmModule, alloc_func: u32) -> u32 {
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([
        (1, ValType::I32), // size
        (1, ValType::I32), // new_ptr
        (1, ValType::I32), // loop counter
    ]);
    
    // Load size from object header (offset 4)
    func.instruction(&Instruction::LocalGet(0)); // object ptr
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(1)); // size
    
    // Allocate new object
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::Call(alloc_func));
    func.instruction(&Instruction::LocalSet(2)); // new_ptr
    
    // Copy bytes: memcpy loop
    // for (i = 0; i < size; i += 4) { new_ptr[i] = old_ptr[i]; }
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::LocalSet(3)); // i = 0
    
    // Loop
    func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    
    // Check if i >= size
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32GeU);
    func.instruction(&Instruction::BrIf(1)); // break if done
    
    // Copy word: new_ptr[i] = old_ptr[i]
    func.instruction(&Instruction::LocalGet(2)); // new_ptr
    func.instruction(&Instruction::LocalGet(3)); // i
    func.instruction(&Instruction::I32Add);
    
    func.instruction(&Instruction::LocalGet(0)); // old_ptr
    func.instruction(&Instruction::LocalGet(3)); // i
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // i += 4
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::I32Const(4));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(3));
    
    func.instruction(&Instruction::Br(0)); // continue loop
    func.instruction(&Instruction::End); // end loop
    func.instruction(&Instruction::End); // end block
    
    // Return new object pointer
    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

// IO methods

/// IO.out_string(s: String) -> IO
///
/// Writes a string to stdout using WASI fd_write.
/// String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
fn add_io_out_string(module: &mut WasmModule, wasi_fd_write: u32) -> u32 {
    // (self: i32, s: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([(1, ValType::I32)]); // local for iovec ptr
    
    // String pointer in local 1 (param)
    // Allocate iovec at a fixed location (offset 0x1000)
    // iovec[0].buf_ptr = string_ptr + 16 (skip header)
    func.instruction(&Instruction::I32Const(0x1000));
    func.instruction(&Instruction::LocalGet(1));  // string ptr
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // iovec[0].buf_len = string length (at offset 12 of string object)
    func.instruction(&Instruction::I32Const(0x1004));
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32Const(12));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // Call fd_write(1, 0x1000, 1, 0x1008)
    func.instruction(&Instruction::I32Const(1));      // stdout fd
    func.instruction(&Instruction::I32Const(0x1000)); // iovec ptr
    func.instruction(&Instruction::I32Const(1));      // iovs_len
    func.instruction(&Instruction::I32Const(0x1008)); // nwritten ptr
    func.instruction(&Instruction::Call(wasi_fd_write));
    func.instruction(&Instruction::Drop);             // drop errno
    
    // Return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.out_int(i: Int) -> IO
///
/// Converts an integer to string and writes to stdout.
fn add_io_out_int(module: &mut WasmModule, wasi_fd_write: u32) -> u32 {
    use wasm_encoder::BlockType;
    
    // (self: i32, i: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    // Buffer for digits at 0x1200, with space for sign and up to 11 digits
    // Locals: value (local 2), is_negative (local 3), idx (local 4), digit (local 5)
    let mut func = Function::new([
        (1, ValType::I32), // value
        (1, ValType::I32), // is_negative  
        (1, ValType::I32), // idx (write position in buffer)
        (1, ValType::I32), // digit
    ]);
    
    let buffer = 0x1200i32;
    
    // value = param 1
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::LocalSet(2));
    
    // Check if negative
    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::I32LtS);
    func.instruction(&Instruction::LocalSet(3)); // is_negative
    
    // If negative, negate value
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::If(BlockType::Empty));
    {
        func.instruction(&Instruction::I32Const(0));
        func.instruction(&Instruction::LocalGet(2));
        func.instruction(&Instruction::I32Sub);
        func.instruction(&Instruction::LocalSet(2));
    }
    func.instruction(&Instruction::End);
    
    // Handle zero case specially
    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::I32Eq);
    func.instruction(&Instruction::If(BlockType::Empty));
    {
        // Write '0' to buffer
        func.instruction(&Instruction::I32Const(buffer));
        func.instruction(&Instruction::I32Const(48)); // '0'
        func.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        func.instruction(&Instruction::I32Const(1));
        func.instruction(&Instruction::LocalSet(4)); // idx = 1
    }
    func.instruction(&Instruction::Else);
    {
        // Start at end of buffer and write digits backwards
        func.instruction(&Instruction::I32Const(0));
        func.instruction(&Instruction::LocalSet(4)); // idx = 0
        
        // Loop: while value > 0, write digit
        func.instruction(&Instruction::Block(BlockType::Empty));
        func.instruction(&Instruction::Loop(BlockType::Empty));
        {
            // if value == 0 then break
            func.instruction(&Instruction::LocalGet(2));
            func.instruction(&Instruction::I32Eqz);
            func.instruction(&Instruction::BrIf(1));
            
            // digit = value % 10
            func.instruction(&Instruction::LocalGet(2));
            func.instruction(&Instruction::I32Const(10));
            func.instruction(&Instruction::I32RemU);
            func.instruction(&Instruction::LocalSet(5));
            
            // buffer[10 - idx] = '0' + digit (write backwards from position 10)
            func.instruction(&Instruction::I32Const(buffer + 10));
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::LocalGet(5));
            func.instruction(&Instruction::I32Const(48)); // '0'
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            
            // idx++
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Const(1));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalSet(4));
            
            // value = value / 10
            func.instruction(&Instruction::LocalGet(2));
            func.instruction(&Instruction::I32Const(10));
            func.instruction(&Instruction::I32DivU);
            func.instruction(&Instruction::LocalSet(2));
            
            func.instruction(&Instruction::Br(0));
        }
        func.instruction(&Instruction::End); // loop
        func.instruction(&Instruction::End); // block
        
        // If negative, prepend '-'
        func.instruction(&Instruction::LocalGet(3));
        func.instruction(&Instruction::If(BlockType::Empty));
        {
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Const(1));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalSet(4));
            
            func.instruction(&Instruction::I32Const(buffer + 10));
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Sub);
            func.instruction(&Instruction::I32Const(45)); // '-'
            func.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
        }
        func.instruction(&Instruction::End);
    }
    func.instruction(&Instruction::End);
    
    // iovec.buf_ptr: for zero case it's buffer, otherwise it's (buffer + 11 - idx)
    func.instruction(&Instruction::I32Const(0x1000)); // iovec location
    func.instruction(&Instruction::I32Const(buffer + 11));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Sub);
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // iovec.buf_len = idx
    func.instruction(&Instruction::I32Const(0x1004));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // Call fd_write(1, 0x1000, 1, 0x1008)
    func.instruction(&Instruction::I32Const(1));      // stdout fd
    func.instruction(&Instruction::I32Const(0x1000)); // iovec ptr
    func.instruction(&Instruction::I32Const(1));      // iovs_len
    func.instruction(&Instruction::I32Const(0x1008)); // nwritten ptr
    func.instruction(&Instruction::Call(wasi_fd_write));
    func.instruction(&Instruction::Drop);             // drop errno
    
    // Return self
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.in_string() -> String
///
/// Reads a string from stdin using WASI fd_read.
/// Returns a simple buffer pointer (not a full String object for MVP).
fn add_io_in_string(module: &mut WasmModule, wasi_fd_read: u32) -> u32 {
    // (self: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([]);
    
    // Set up iovec to read into buffer at 0x3000
    // iovec[0].buf_ptr = 0x3000
    func.instruction(&Instruction::I32Const(0x1100));
    func.instruction(&Instruction::I32Const(0x3000));
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // iovec[0].buf_len = 1024 bytes
    func.instruction(&Instruction::I32Const(0x1104));
    func.instruction(&Instruction::I32Const(1024));
    func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // Call fd_read(0, 0x1100, 1, 0x1108)
    func.instruction(&Instruction::I32Const(0));      // stdin fd
    func.instruction(&Instruction::I32Const(0x1100)); // iovec ptr
    func.instruction(&Instruction::I32Const(1));      // iovs_len
    func.instruction(&Instruction::I32Const(0x1108)); // nread ptr
    func.instruction(&Instruction::Call(wasi_fd_read));
    func.instruction(&Instruction::Drop);             // drop errno
    
    // TODO: Allocate proper String object
    // For MVP: return buffer pointer
    func.instruction(&Instruction::I32Const(0x3000));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// IO.in_int() -> Int
///
/// Reads a string from stdin and parses it as an integer.
/// Simplified implementation for MVP.
fn add_io_in_int(module: &mut WasmModule, _wasi_fd_read: u32) -> u32 {
    // (self: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    // For MVP: stub - return 0
    // TODO: Implement proper parsing
    let mut func = Function::new([]);
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
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
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
/// String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
fn add_string_concat(module: &mut WasmModule, alloc_func: u32) -> u32 {
    // (self: i32, s: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([
        (1, ValType::I32), // len1
        (1, ValType::I32), // len2
        (1, ValType::I32), // new_len
        (1, ValType::I32), // new_ptr
        (1, ValType::I32), // loop counter
    ]);
    
    // Get length of first string (offset 12)
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(2)); // len1
    
    // Get length of second string
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(3)); // len2
    
    // Calculate new length
    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(4)); // new_len
    
    // Allocate new string: header (16 bytes) + data
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::Call(alloc_func));
    func.instruction(&Instruction::LocalSet(5)); // new_ptr
    
    // Copy class_tag from first string
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // Store size (header + data)
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    
    // Copy vtable_ptr from first string
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    
    // Store new length
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    
    // Copy first string data (byte by byte)
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::LocalSet(6)); // i = 0
    
    func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::LocalGet(2)); // len1
    func.instruction(&Instruction::I32GeU);
    func.instruction(&Instruction::BrIf(1));
    
    // new_ptr[16 + i] = str1[16 + i]
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Const(1));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(6));
    func.instruction(&Instruction::Br(0));
    func.instruction(&Instruction::End);
    func.instruction(&Instruction::End);
    
    // Copy second string data
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::LocalSet(6)); // i = 0
    
    func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::LocalGet(3)); // len2
    func.instruction(&Instruction::I32GeU);
    func.instruction(&Instruction::BrIf(1));
    
    // new_ptr[16 + len1 + i] = str2[16 + i]
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(2)); // len1
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Const(1));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(6));
    func.instruction(&Instruction::Br(0));
    func.instruction(&Instruction::End);
    func.instruction(&Instruction::End);
    
    // Return new string
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// String.substr(i: Int, l: Int) -> String
///
/// Returns a substring starting at position i with length l.
/// String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
fn add_string_substr(module: &mut WasmModule, alloc_func: u32) -> u32 {
    // (self: i32, i: i32, l: i32) -> i32
    let type_idx = module.add_type(
        vec![ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );
    let func_idx = module.add_function(type_idx);

    let mut func = Function::new([
        (1, ValType::I32), // str_len
        (1, ValType::I32), // actual_len
        (1, ValType::I32), // new_ptr
        (1, ValType::I32), // loop counter
    ]);
    
    // Get string length
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(3)); // str_len
    
    // Calculate actual substring length: min(l, str_len - i)
    // For MVP: assume valid bounds, just use l
    func.instruction(&Instruction::LocalGet(2)); // l
    func.instruction(&Instruction::LocalSet(4)); // actual_len = l
    
    // Allocate new string
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::Call(alloc_func));
    func.instruction(&Instruction::LocalSet(5)); // new_ptr
    
    // Copy class_tag
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    
    // Store size
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    
    // Copy vtable_ptr
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 8,
        align: 2,
        memory_index: 0,
    }));
    
    // Store length
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::LocalGet(4));
    func.instruction(&Instruction::I32Store(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    
    // Copy substring data
    func.instruction(&Instruction::I32Const(0));
    func.instruction(&Instruction::LocalSet(6)); // loop counter = 0
    
    func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::LocalGet(4)); // actual_len
    func.instruction(&Instruction::I32GeU);
    func.instruction(&Instruction::BrIf(1));
    
    // new_ptr[16 + loop_counter] = old_ptr[16 + i + loop_counter]
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Const(16));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(1)); // i
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::I32Load8U(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    func.instruction(&Instruction::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    
    func.instruction(&Instruction::LocalGet(6));
    func.instruction(&Instruction::I32Const(1));
    func.instruction(&Instruction::I32Add);
    func.instruction(&Instruction::LocalSet(6));
    func.instruction(&Instruction::Br(0));
    func.instruction(&Instruction::End);
    func.instruction(&Instruction::End);
    
    // Return new string
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}

/// String_equals(s1: String, s2: String) -> Bool (i32)
///
/// Compares two strings by content.
/// Returns 1 (true) if equal, 0 (false) otherwise.
fn add_string_equals(module: &mut WasmModule) -> u32 {
    use wasm_encoder::BlockType;
    
    // (s1: i32, s2: i32) -> i32
    let type_idx = module.add_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    let func_idx = module.add_function(type_idx);

    // Locals: len1, len2, idx, result
    let mut func = Function::new([
        (1, ValType::I32), // len1 (local 2)
        (1, ValType::I32), // len2 (local 3)
        (1, ValType::I32), // idx (local 4)
        (1, ValType::I32), // result (local 5)
    ]);
    
    // Get length of first string (offset 12)
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(2)); // len1
    
    // Get length of second string
    func.instruction(&Instruction::LocalGet(1));
    func.instruction(&Instruction::I32Load(MemArg {
        offset: 12,
        align: 2,
        memory_index: 0,
    }));
    func.instruction(&Instruction::LocalSet(3)); // len2
    
    // If lengths differ, return false
    func.instruction(&Instruction::LocalGet(2));
    func.instruction(&Instruction::LocalGet(3));
    func.instruction(&Instruction::I32Ne);
    func.instruction(&Instruction::If(BlockType::Empty));
    {
        func.instruction(&Instruction::I32Const(0)); // false
        func.instruction(&Instruction::LocalSet(5)); // result = false
    }
    func.instruction(&Instruction::Else);
    {
        // Default: assume equal
        func.instruction(&Instruction::I32Const(1));
        func.instruction(&Instruction::LocalSet(5)); // result = true
        
        // Compare character by character
        func.instruction(&Instruction::I32Const(0));
        func.instruction(&Instruction::LocalSet(4)); // idx = 0
        
        func.instruction(&Instruction::Block(BlockType::Empty)); // outer block for early exit
        func.instruction(&Instruction::Loop(BlockType::Empty));
        {
            // if idx >= len1 then exit loop (done)
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::LocalGet(2));
            func.instruction(&Instruction::I32GeU);
            func.instruction(&Instruction::BrIf(1)); // exit outer block
            
            // Compare s1[idx] with s2[idx]
            func.instruction(&Instruction::LocalGet(0));
            func.instruction(&Instruction::I32Const(16)); // skip header
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            
            func.instruction(&Instruction::LocalGet(1));
            func.instruction(&Instruction::I32Const(16));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::I32Load8U(MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            
            func.instruction(&Instruction::I32Ne);
            func.instruction(&Instruction::If(BlockType::Empty));
            {
                // Characters differ, set result to false and exit
                func.instruction(&Instruction::I32Const(0));
                func.instruction(&Instruction::LocalSet(5));
                func.instruction(&Instruction::Br(2)); // exit outer block
            }
            func.instruction(&Instruction::End);
            
            // idx++
            func.instruction(&Instruction::LocalGet(4));
            func.instruction(&Instruction::I32Const(1));
            func.instruction(&Instruction::I32Add);
            func.instruction(&Instruction::LocalSet(4));
            
            func.instruction(&Instruction::Br(0)); // continue loop
        }
        func.instruction(&Instruction::End); // loop
        func.instruction(&Instruction::End); // block
    }
    func.instruction(&Instruction::End); // if/else
    
    // Return result
    func.instruction(&Instruction::LocalGet(5));
    func.instruction(&Instruction::End);

    module.add_code(func);
    func_idx
}
