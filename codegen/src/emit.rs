//! IR to WASM instruction emission
//!
//! This module converts from the intermediate representation (LIR)
//! to WASM instructions using wasm-encoder.

use crate::runtime::{add_runtime, RuntimeFunctions};
use crate::wasm::WasmModule;
use ir::lir::*;
use std::collections::HashMap;
use wasm_encoder::{Function, Instruction, ValType};

/// Emit context for tracking function-local state during code generation
pub struct EmitContext {
    /// Function name to index mapping
    function_map: HashMap<String, u32>,
    /// Label to depth mapping (for br/br_if instructions)
    label_depths: HashMap<String, u32>,
    /// Current label depth
    current_depth: u32,
    /// Runtime function indices
    runtime: RuntimeFunctions,
}

impl EmitContext {
    /// Create a new emission context with runtime function indices
    pub fn new(runtime: RuntimeFunctions) -> Self {
        Self {
            function_map: HashMap::new(),
            label_depths: HashMap::new(),
            current_depth: 0,
            runtime,
        }
    }

    /// Get runtime function indices
    pub fn runtime(&self) -> &RuntimeFunctions {
        &self.runtime
    }

    /// Register a function name -> index mapping
    pub fn register_function(&mut self, name: String, idx: u32) {
        self.function_map.insert(name, idx);
    }

    /// Get function index by name
    pub fn get_function(&self, name: &str) -> Option<u32> {
        self.function_map.get(name).copied()
    }

    /// Push a label scope
    pub fn push_label(&mut self, label: String) {
        self.label_depths.insert(label, self.current_depth);
        self.current_depth += 1;
    }

    /// Pop a label scope
    pub fn pop_label(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }

    /// Get label depth for branching
    pub fn get_label_depth(&self, label: &str) -> Option<u32> {
        self.label_depths.get(label).map(|&depth| self.current_depth - depth - 1)
    }
}

impl Default for EmitContext {
    fn default() -> Self {
        // Create dummy runtime for default (shouldn't be used in practice)
        Self::new(RuntimeFunctions {
            alloc: 0,
            object_abort: 0,
            object_type_name: 0,
            object_copy: 0,
            io_out_string: 0,
            io_out_int: 0,
            io_in_string: 0,
            io_in_int: 0,
            string_length: 0,
            string_concat: 0,
            string_substr: 0,
            wasi_fd_write: 0,
            wasi_fd_read: 0,
        })
    }
}

/// Emit a complete WASM module from LIR program
pub fn emit_module(program: &LirProgram, hir: &ir::hir::HirProgram) -> Result<Vec<u8>, String> {
    let mut module = WasmModule::new();

    // Initialize memory (1 page = 64KB for static data + heap)
    module.init_memory(16); // 16 pages = 1MB

    // Export memory for debugging
    module.export_memory("memory");

    // Add string literals to data section
    // String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
    for string_data in &program.string_data {
        let bytes = string_data.value.as_bytes();
        let length = bytes.len() as i32;
        
        // Build string object in memory: header + data
        let mut data = Vec::new();
        
        // class_tag (String class tag - use 0 for now)
        data.extend_from_slice(&0i32.to_le_bytes());
        
        // size (total size of object)
        let total_size = 16 + bytes.len();
        data.extend_from_slice(&(total_size as i32).to_le_bytes());
        
        // vtable_ptr (0 for now)
        data.extend_from_slice(&0i32.to_le_bytes());
        
        // length (number of characters)
        data.extend_from_slice(&length.to_le_bytes());
        
        // string data
        data.extend_from_slice(bytes);
        
        module.add_data(string_data.offset, data);
    }

    // Add runtime functions (Object, IO, String)
    let runtime = add_runtime(&mut module);
    
    let mut ctx = EmitContext::new(runtime);

    // Register all program functions first (for call references)
    for (i, func) in program.functions.iter().enumerate() {
        // Runtime functions take first indices, so offset by runtime count
        // 2 WASI imports + 1 alloc + 3 Object + 4 IO + 3 String = 13 functions
        let func_idx = (i + 13) as u32;
        ctx.register_function(func.name.clone(), func_idx);
    }
    
    // Register built-in class methods to runtime functions
    ctx.register_function("IO_out_string".to_string(), runtime.io_out_string);
    ctx.register_function("IO_out_int".to_string(), runtime.io_out_int);
    ctx.register_function("IO_in_string".to_string(), runtime.io_in_string);
    ctx.register_function("IO_in_int".to_string(), runtime.io_in_int);
    ctx.register_function("Object_abort".to_string(), runtime.object_abort);
    ctx.register_function("Object_type_name".to_string(), runtime.object_type_name);
    ctx.register_function("Object_copy".to_string(), runtime.object_copy);
    ctx.register_function("String_length".to_string(), runtime.string_length);
    ctx.register_function("String_concat".to_string(), runtime.string_concat);
    ctx.register_function("String_substr".to_string(), runtime.string_substr);

    // Add type signatures and function declarations
    for func in &program.functions {
        let params: Vec<ValType> = func.params.iter()
            .map(|t| lir_type_to_wasm(t))
            .collect();
        let results: Vec<ValType> = match &func.return_type {
            LirType::Void => vec![],
            t => vec![lir_type_to_wasm(t)],
        };

        let type_idx = module.add_type(params, results);
        module.add_function(type_idx);
    }

    // Emit function bodies
    for func in &program.functions {
        let wasm_func = emit_function(&ctx, func)?;
        module.add_code(wasm_func);
    }

    // Create _start wrapper function for WASI
    // WASI requires _start to have signature () -> ()
    if let Some(main_idx) = ctx.get_function("Main_main") {
        // Find Main class in HIR to get class_tag and size
        let main_class = hir.classes.iter().find(|c| c.name == "Main");
        
        if let Some(main_class) = main_class {
            let start_type = module.add_type(vec![], vec![]);
            let start_idx = module.add_function(start_type);
            
            // Calculate Main object size:
            // Header: 12 bytes (class_tag: 4, size: 4, vtable_ptr: 4)
            // Attributes: 4 bytes each
            let obj_size = 12 + (main_class.attributes.len() * 4);
            
            // Local 0: object pointer (i32)
            let mut start_func = Function::new([(1, ValType::I32)]);
            
            // Allocate Main object: call $alloc with size
            start_func.instruction(&Instruction::I32Const(obj_size as i32));
            start_func.instruction(&Instruction::Call(ctx.runtime().alloc));
            
            // Initialize object header
            // Store class_tag at offset 0
            start_func.instruction(&Instruction::LocalTee(0)); // Save pointer
            start_func.instruction(&Instruction::I32Const(main_class.class_tag as i32));
            start_func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 0,
                align: 2,
                memory_index: 0,
            }));
            
            // Store size at offset 4
            start_func.instruction(&Instruction::LocalGet(0));
            start_func.instruction(&Instruction::I32Const(obj_size as i32));
            start_func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 4,
                align: 2,
                memory_index: 0,
            }));
            
            // Store vtable_ptr at offset 8 (0 for now, will be set when vtables are implemented)
            start_func.instruction(&Instruction::LocalGet(0));
            start_func.instruction(&Instruction::I32Const(0));
            start_func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: 8,
                align: 2,
                memory_index: 0,
            }));
            
            // Call Main.main() with the object pointer - it returns SELF_TYPE (i32 pointer)
            start_func.instruction(&Instruction::LocalGet(0));
            start_func.instruction(&Instruction::Call(main_idx));
            
            // Drop the return value
            start_func.instruction(&Instruction::Drop);
            start_func.instruction(&Instruction::End);
            
            module.add_code(start_func);
            module.export_function("_start", start_idx);
        }
    }

    // TODO: Emit vtables in data section
    // TODO: Emit string constants in data section

    Ok(module.finish())
}

/// Convert LirType to WASM ValType
fn lir_type_to_wasm(typ: &LirType) -> ValType {
    match typ {
        LirType::I32 => ValType::I32,
        LirType::I64 => ValType::I64,
        LirType::Void => panic!("Void type should not be converted to ValType"),
    }
}

/// Emit a function body from LIR
fn emit_function(ctx: &EmitContext, lir_func: &LirFunction) -> Result<Function, String> {
    // Calculate local types (excluding parameters)
    let param_count = lir_func.params.len();
    let local_types: Vec<(u32, ValType)> = lir_func.locals.iter()
        .filter(|l| (l.index as usize) >= param_count)
        .map(|l| (1u32, lir_type_to_wasm(&l.typ)))
        .collect();

    let mut func = Function::new(local_types);

    // Emit instructions
    for instr in &lir_func.body {
        emit_instruction(&mut func, ctx, instr)?;
    }

    // Add implicit return if missing
    func.instruction(&Instruction::End);

    Ok(func)
}

/// Emit a single LIR instruction to WASM
fn emit_instruction(func: &mut Function, ctx: &EmitContext, instr: &LirInstr) -> Result<(), String> {
    match instr {
        LirInstr::I32Const(val) => {
            func.instruction(&Instruction::I32Const(*val));
        }

        LirInstr::Dup => {
            // WASM doesn't have dup, so use local.tee with a temp local
            // For simplicity, skip for now (would need temp local allocation)
        }

        LirInstr::Drop => {
            func.instruction(&Instruction::Drop);
        }

        LirInstr::I32Add => {
            func.instruction(&Instruction::I32Add);
        }

        LirInstr::I32Sub => {
            func.instruction(&Instruction::I32Sub);
        }

        LirInstr::I32Mul => {
            func.instruction(&Instruction::I32Mul);
        }

        LirInstr::I32DivS => {
            func.instruction(&Instruction::I32DivS);
        }

        LirInstr::I32Neg => {
            // Negate is 0 - x, but should already be lowered
            // If we see it, emit as is (no direct instruction)
        }

        LirInstr::I32LtS => {
            func.instruction(&Instruction::I32LtS);
        }

        LirInstr::I32LeS => {
            func.instruction(&Instruction::I32LeS);
        }

        LirInstr::I32Eq => {
            func.instruction(&Instruction::I32Eq);
        }

        LirInstr::I32Ne => {
            func.instruction(&Instruction::I32Ne);
        }

        LirInstr::I32Eqz => {
            func.instruction(&Instruction::I32Eqz);
        }

        LirInstr::LocalGet(idx) => {
            func.instruction(&Instruction::LocalGet(*idx));
        }

        LirInstr::LocalSet(idx) => {
            func.instruction(&Instruction::LocalSet(*idx));
        }

        LirInstr::LocalTee(idx) => {
            func.instruction(&Instruction::LocalTee(*idx));
        }

        LirInstr::GlobalGet(idx) => {
            func.instruction(&Instruction::GlobalGet(*idx));
        }

        LirInstr::GlobalSet(idx) => {
            func.instruction(&Instruction::GlobalSet(*idx));
        }

        LirInstr::I32Load { offset, align } => {
            func.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
                offset: (*offset).into(),
                align: align.trailing_zeros(),
                memory_index: 0,
            }));
        }

        LirInstr::I32Store { offset, align } => {
            func.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
                offset: (*offset).into(),
                align: align.trailing_zeros(),
                memory_index: 0,
            }));
        }

        LirInstr::Alloc => {
            // Call allocator function (size is on stack)
            func.instruction(&Instruction::Call(ctx.runtime().alloc));
        }

        LirInstr::Label(_label) => {
            // Labels are handled by structured control flow, ignore
        }

        LirInstr::Jump(label) => {
            // Convert to br with calculated depth
            if let Some(depth) = ctx.get_label_depth(&label.0) {
                func.instruction(&Instruction::Br(depth));
            }
        }

        LirInstr::JumpIf(label) => {
            // Branch if top of stack is non-zero
            if let Some(depth) = ctx.get_label_depth(&label.0) {
                func.instruction(&Instruction::BrIf(depth));
            }
        }

        LirInstr::JumpIfNot(label) => {
            // Branch if top of stack is zero
            func.instruction(&Instruction::I32Eqz);
            if let Some(depth) = ctx.get_label_depth(&label.0) {
                func.instruction(&Instruction::BrIf(depth));
            }
        }

        LirInstr::Block { .. } => {
            func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        }

        LirInstr::End => {
            func.instruction(&Instruction::End);
        }

        LirInstr::Loop { .. } => {
            func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        }

        LirInstr::Br(depth) => {
            func.instruction(&Instruction::Br(*depth));
        }

        LirInstr::BrIf(depth) => {
            func.instruction(&Instruction::BrIf(*depth));
        }

        LirInstr::Call(name) => {
            if let Some(func_idx) = ctx.get_function(name) {
                func.instruction(&Instruction::Call(func_idx));
            } else {
                return Err(format!("Unknown function: {}", name));
            }
        }

        LirInstr::CallIndirect { type_index, .. } => {
            func.instruction(&Instruction::CallIndirect {
                type_index: *type_index,
                table_index: 0,
            });
        }

        LirInstr::Return => {
            func.instruction(&Instruction::Return);
        }

        LirInstr::Unreachable => {
            func.instruction(&Instruction::Unreachable);
        }

        LirInstr::Nop => {
            func.instruction(&Instruction::Nop);
        }

        LirInstr::Comment(_) => {
            // Comments are ignored in binary emission
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_context() {
        // Create a dummy runtime
        let runtime = crate::runtime::RuntimeFunctions {
            alloc: 2,
            object_abort: 3,
            object_type_name: 4,
            object_copy: 5,
            io_out_string: 6,
            io_out_int: 7,
            io_in_string: 8,
            io_in_int: 9,
            string_length: 10,
            string_concat: 11,
            string_substr: 12,
            wasi_fd_write: 0,
            wasi_fd_read: 1,
        };
        let mut ctx = EmitContext::new(runtime);
        ctx.register_function("test_func".to_string(), 42);
        assert_eq!(ctx.get_function("test_func"), Some(42));
    }

    #[test]
    fn test_lir_type_conversion() {
        assert_eq!(lir_type_to_wasm(&LirType::I32), ValType::I32);
        assert_eq!(lir_type_to_wasm(&LirType::I64), ValType::I64);
    }
}
