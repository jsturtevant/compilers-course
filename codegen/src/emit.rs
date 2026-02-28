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
    /// Type index for call_indirect by number of params (params -> type_idx)
    call_types: HashMap<u32, u32>,
    /// Map from class name to vtable address in memory
    vtable_addresses: HashMap<String, u32>,
    /// Address of pre-allocated Bool false constant
    bool_false_addr: u32,
    /// Address of pre-allocated Bool true constant
    bool_true_addr: u32,
}

impl EmitContext {
    /// Create a new emission context with runtime function indices
    pub fn new(runtime: RuntimeFunctions) -> Self {
        Self {
            function_map: HashMap::new(),
            label_depths: HashMap::new(),
            current_depth: 0,
            runtime,
            call_types: HashMap::new(),
            vtable_addresses: HashMap::new(),
            bool_false_addr: 0,
            bool_true_addr: 0,
        }
    }

    /// Get runtime function indices
    pub fn runtime(&self) -> &RuntimeFunctions {
        &self.runtime
    }

    /// Set Bool constant addresses
    pub fn set_bool_addrs(&mut self, false_addr: u32, true_addr: u32) {
        self.bool_false_addr = false_addr;
        self.bool_true_addr = true_addr;
    }

    /// Get Bool false constant address
    pub fn get_bool_false_addr(&self) -> u32 {
        self.bool_false_addr
    }

    /// Get Bool true constant address
    pub fn get_bool_true_addr(&self) -> u32 {
        self.bool_true_addr
    }

    /// Register a function name -> index mapping
    pub fn register_function(&mut self, name: String, idx: u32) {
        self.function_map.insert(name, idx);
    }

    /// Get function index by name
    pub fn get_function(&self, name: &str) -> Option<u32> {
        self.function_map.get(name).copied()
    }

    /// Register a type index for call_indirect with given param count
    pub fn register_call_type(&mut self, num_params: u32, type_idx: u32) {
        self.call_types.insert(num_params, type_idx);
    }

    /// Get type index for call_indirect with given param count
    pub fn get_call_type(&self, num_params: u32) -> Option<u32> {
        self.call_types.get(&num_params).copied()
    }

    /// Register vtable address for a class
    pub fn register_vtable_address(&mut self, class_name: String, addr: u32) {
        self.vtable_addresses.insert(class_name, addr);
    }

    /// Get vtable address for a class
    pub fn get_vtable_address(&self, class_name: &str) -> Option<u32> {
        self.vtable_addresses.get(class_name).copied()
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
            string_equals: 0,
            wasi_fd_write: 0,
            wasi_fd_read: 0,
        })
    }
}

/// Emit a complete WASM module from LIR program
pub fn emit_module(program: &LirProgram, hir: &ir::hir::HirProgram) -> Result<Vec<u8>, String> {
    let mut module = WasmModule::new();

    // Initialize memory (1 page = 64KB for static data + heap)
    module.init_memory(256); // 256 pages = 16MB

    // Export memory for debugging
    module.export_memory("memory");

    // Collect all class names (builtins first, then user classes sorted by class_tag)
    // Builtin class tags: Object=0, IO=1, String=2, Int=3, Bool=4
    let builtin_class_names = vec!["Object", "IO", "String", "Int", "Bool"];
    let mut all_class_names: Vec<(usize, &str)> = builtin_class_names
        .iter()
        .enumerate()
        .map(|(i, name)| (i, *name))
        .collect();
    
    // Add user classes from HIR
    for class in &hir.classes {
        // Skip if it's a builtin (class_tag < 5)
        if class.class_tag >= 5 {
            all_class_names.push((class.class_tag, &class.name));
        }
    }
    // Sort by class_tag to ensure correct ordering
    all_class_names.sort_by_key(|(tag, _)| *tag);
    
    // Calculate memory layout for class name table
    // Class name table at 0x1800 (between iovec at 0x1000 and strings at 0x2000)
    let class_name_table_addr: u32 = 0x1800;
    let num_classes = all_class_names.len();
    // Each entry is a 4-byte pointer
    let class_name_table_size = (num_classes * 4) as u32;
    // String objects start after the table, aligned
    let class_name_strings_start = ((class_name_table_addr + class_name_table_size + 15) / 16) * 16;

    // Calculate heap start address (must be after all static data)
    // Layout: strings at 0x2000, vtables after strings
    let num_strings = program.string_data.len() as u32;
    let strings_end = 0x2000u32 + num_strings * 128;
    let mut vtable_offset = ((strings_end + 255) / 256) * 256;
    if vtable_offset < 0x4000 {
        vtable_offset = 0x4000; // Minimum address for vtables
    }
    
    // Calculate vtable size: builtin vtables + user vtables
    // Object(3) + IO(7) + String(6) + Int(3) + Bool(3) = 22
    let builtin_vtable_entries = 3 + 7 + 6 + 3 + 3;
    let user_vtable_entries: usize = program.vtables.iter()
        .map(|v| v.methods.len())
        .sum();
    let vtables_size = ((builtin_vtable_entries + user_vtable_entries) * 4) as u32;
    
    // Heap starts after vtables, aligned to 256-byte boundary
    let heap_start = ((vtable_offset + vtables_size + 255) / 256) * 256;

    // Calculate String vtable address: Object(3*4) + IO(7*4) = 40 bytes offset
    let string_vtable_addr = vtable_offset + 3 * 4 + 7 * 4;

    // Add runtime functions (Object, IO, String), passing class name table address
    let runtime = add_runtime(&mut module, heap_start, class_name_table_addr, string_vtable_addr);
    
    let mut ctx = EmitContext::new(runtime);

    // Register all program functions first (for call references)
    for (i, func) in program.functions.iter().enumerate() {
        // Runtime functions take first indices, so offset by runtime count
        // 2 WASI imports + 1 alloc + 3 Object + 4 IO + 4 String = 14 functions
        let func_idx = (i + 14) as u32;
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
    ctx.register_function("String_equals".to_string(), runtime.string_equals);

    // Add type signatures for call_indirect with different param counts (0-10 params)
    // All COOL methods take i32s and return i32
    for num_params in 0..=10 {
        let params: Vec<ValType> = (0..num_params).map(|_| ValType::I32).collect();
        let type_idx = module.add_type(params, vec![ValType::I32]);
        ctx.register_call_type(num_params, type_idx);
    }

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

    // Define builtin vtables (alphabetically sorted methods, matching class_hierarchy.get_all_methods)
    let builtin_vtables = vec![
        ("Object", vec!["Object_abort", "Object_copy", "Object_type_name"]),
        ("IO", vec!["Object_abort", "Object_copy", "Object_type_name", "IO_in_int", "IO_in_string", "IO_out_int", "IO_out_string"]),
        ("String", vec!["Object_abort", "Object_copy", "Object_type_name", "String_concat", "String_length", "String_substr"]),
        ("Int", vec!["Object_abort", "Object_copy", "Object_type_name"]),
        ("Bool", vec!["Object_abort", "Object_copy", "Object_type_name"]),
    ];

    // Initialize function table for dynamic dispatch
    // Collect all functions that need to be in the table (from both builtin and user vtables)
    let mut table_functions: Vec<u32> = Vec::new();
    
    // Add builtin vtable methods to function table
    for (_class_name, methods) in &builtin_vtables {
        for method_name in methods {
            if let Some(func_idx) = ctx.get_function(method_name) {
                if !table_functions.contains(&func_idx) {
                    table_functions.push(func_idx);
                }
            }
        }
    }
    
    // Add user-defined vtable methods to function table
    for vtable in &program.vtables {
        for method_name in &vtable.methods {
            if let Some(func_idx) = ctx.get_function(method_name) {
                if !table_functions.contains(&func_idx) {
                    table_functions.push(func_idx);
                }
            }
        }
    }
    
    // Only create table if we have functions to put in it
    if !table_functions.is_empty() {
        let table_size = table_functions.len() as u32;
        module.init_table(table_size, Some(table_size));
        module.add_element_section(&table_functions);
    }

    // Create vtables in memory and register their addresses
    // Vtables start after string literals, aligned to 256-byte boundary
    // Re-calculate vtable_offset based on actual string data
    let num_strings = program.string_data.len() as u32;
    let strings_end = 0x2000u32 + num_strings * 128;
    let mut vtable_offset = ((strings_end + 255) / 256) * 256;
    if vtable_offset < 0x4000 {
        vtable_offset = 0x4000; // Minimum address for vtables
    }
    
    // First, create builtin vtables
    for (class_name, methods) in &builtin_vtables {
        ctx.register_vtable_address(class_name.to_string(), vtable_offset);
        
        let mut vtable_data = Vec::new();
        for method_name in methods {
            let table_idx = if let Some(func_idx) = ctx.get_function(method_name) {
                table_functions.iter().position(|&f| f == func_idx).unwrap_or(0) as u32
            } else {
                0
            };
            vtable_data.extend_from_slice(&table_idx.to_le_bytes());
        }
        
        if !vtable_data.is_empty() {
            module.add_data(vtable_offset, vtable_data.clone());
        }
        vtable_offset += (methods.len() * 4) as u32;
    }
    
    // Then, create user-defined vtables
    for vtable in &program.vtables {
        ctx.register_vtable_address(vtable.class_name.clone(), vtable_offset);
        
        // Build vtable data: array of function table indices
        let mut vtable_data = Vec::new();
        for method_name in &vtable.methods {
            // Find the index in the function table (not the function index)
            let table_idx = if let Some(func_idx) = ctx.get_function(method_name) {
                table_functions.iter().position(|&f| f == func_idx).unwrap_or(0) as u32
            } else {
                0
            };
            vtable_data.extend_from_slice(&table_idx.to_le_bytes());
        }
        
        if !vtable_data.is_empty() {
            module.add_data(vtable_offset, vtable_data.clone());
        }
        vtable_offset += (vtable.methods.len() * 4) as u32;
    }

    // Create static Bool objects (BOOL_FALSE and BOOL_TRUE) after vtables
    // Bool object layout: class_tag:i32, size:i32, vtable:i32, value:i32
    let bool_vtable_addr = ctx.get_vtable_address("Bool").unwrap_or(0);
    
    // Align to 16-byte boundary for Bool objects
    let bool_false_addr = ((vtable_offset + 15) / 16) * 16;
    let bool_true_addr = bool_false_addr + 16;
    
    // Create BOOL_FALSE object (value = 0)
    let mut bool_false_data = Vec::new();
    bool_false_data.extend_from_slice(&4i32.to_le_bytes());  // class_tag = 4 (Bool)
    bool_false_data.extend_from_slice(&16i32.to_le_bytes()); // size = 16
    bool_false_data.extend_from_slice(&(bool_vtable_addr as i32).to_le_bytes()); // vtable_ptr
    bool_false_data.extend_from_slice(&0i32.to_le_bytes());  // value = 0 (false)
    module.add_data(bool_false_addr, bool_false_data);
    
    // Create BOOL_TRUE object (value = 1)
    let mut bool_true_data = Vec::new();
    bool_true_data.extend_from_slice(&4i32.to_le_bytes());  // class_tag = 4 (Bool)
    bool_true_data.extend_from_slice(&16i32.to_le_bytes()); // size = 16
    bool_true_data.extend_from_slice(&(bool_vtable_addr as i32).to_le_bytes()); // vtable_ptr
    bool_true_data.extend_from_slice(&1i32.to_le_bytes());  // value = 1 (true)
    module.add_data(bool_true_addr, bool_true_data);
    
    // Register Bool constant addresses in context
    ctx.set_bool_addrs(bool_false_addr, bool_true_addr);

    // Create class name String objects and lookup table for Object.type_name()
    // String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
    let string_vtable_addr = ctx.get_vtable_address("String").unwrap_or(0);
    
    // Build the class name String objects
    let mut current_string_addr = class_name_strings_start;
    let mut class_name_string_addrs: Vec<u32> = Vec::new();
    
    for (_class_tag, class_name) in &all_class_names {
        let bytes = class_name.as_bytes();
        let length = bytes.len() as i32;
        let total_size = 16 + bytes.len();
        
        let mut data = Vec::new();
        data.extend_from_slice(&2i32.to_le_bytes()); // class_tag = 2 (String)
        data.extend_from_slice(&(total_size as i32).to_le_bytes()); // size
        data.extend_from_slice(&(string_vtable_addr as i32).to_le_bytes()); // vtable_ptr
        data.extend_from_slice(&length.to_le_bytes()); // length
        data.extend_from_slice(bytes); // data
        
        module.add_data(current_string_addr, data);
        class_name_string_addrs.push(current_string_addr);
        
        // Align next string to 4-byte boundary
        current_string_addr += ((total_size as u32 + 3) / 4) * 4;
    }
    
    // Build the class name lookup table (array of pointers)
    let mut table_data = Vec::new();
    for addr in &class_name_string_addrs {
        table_data.extend_from_slice(&(*addr as i32).to_le_bytes());
    }
    module.add_data(class_name_table_addr, table_data);

    // Add string literals to data section AFTER vtable addresses are known
    // String layout: [class_tag:i32, size:i32, vtable:i32, length:i32, data:u8...]
    let string_vtable_addr = ctx.get_vtable_address("String").unwrap_or(0);
    for string_data in &program.string_data {
        let bytes = string_data.value.as_bytes();
        let length = bytes.len() as i32;
        
        // Build string object in memory: header + data
        let mut data = Vec::new();
        
        // class_tag (String class tag = 2)
        data.extend_from_slice(&2i32.to_le_bytes());
        
        // size (total size of object)
        let total_size = 16 + bytes.len();
        data.extend_from_slice(&(total_size as i32).to_le_bytes());
        
        // vtable_ptr (String's vtable address)
        data.extend_from_slice(&(string_vtable_addr as i32).to_le_bytes());
        
        // length (number of characters)
        data.extend_from_slice(&length.to_le_bytes());
        
        // string data
        data.extend_from_slice(bytes);
        
        module.add_data(string_data.offset, data);
    }

    // Emit function bodies
    for func in &program.functions {
        let wasm_func = emit_function(&ctx, func)?;
        module.add_code(wasm_func);
    }

    // Create _start wrapper function for WASI
    // WASI requires _start to have signature () -> ()
    if let Some(main_idx) = ctx.get_function("Main_main") {
        // Check if we have a Main_new function (generated by LIR lowering)
        if let Some(main_new_idx) = ctx.get_function("Main_new") {
            let start_type = module.add_type(vec![], vec![]);
            let start_idx = module.add_function(start_type);
            
            // Local 0: object pointer (i32)
            let mut start_func = Function::new([(1, ValType::I32)]);
            
            // Call Main_new() to create and initialize Main object
            start_func.instruction(&Instruction::Call(main_new_idx));
            start_func.instruction(&Instruction::LocalSet(0));
            
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

        LirInstr::I32Or => {
            func.instruction(&Instruction::I32Or);
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

        // Structured control flow
        LirInstr::IfElse { then_instrs, else_instrs, result_type } => {
            // WASM if/else block
            let block_type = match result_type {
                Some(LirType::I32) => wasm_encoder::BlockType::Result(ValType::I32),
                Some(LirType::I64) => wasm_encoder::BlockType::Result(ValType::I64),
                Some(LirType::Void) | None => wasm_encoder::BlockType::Empty,
            };
            
            func.instruction(&Instruction::If(block_type));
            for instr in then_instrs {
                emit_instruction(func, ctx, instr)?;
            }
            func.instruction(&Instruction::Else);
            for instr in else_instrs {
                emit_instruction(func, ctx, instr)?;
            }
            func.instruction(&Instruction::End);
        }

        LirInstr::WhileLoop { cond_instrs, body_instrs } => {
            // WASM while pattern: block { loop { cond; i32.eqz; br_if 1; body; drop; br 0 } } i32.const 0
            // Outer block (for breaking out of the loop)
            func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            // Inner loop (for continuing the loop)
            func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            
            // Evaluate condition
            for instr in cond_instrs {
                emit_instruction(func, ctx, instr)?;
            }
            
            // Branch out of loop if condition is false (i.e., condition == 0)
            func.instruction(&Instruction::I32Eqz);
            func.instruction(&Instruction::BrIf(1)); // Break out of block (depth 1)
            
            // Loop body
            for instr in body_instrs {
                emit_instruction(func, ctx, instr)?;
            }
            func.instruction(&Instruction::Drop); // Discard body result
            
            // Branch back to loop start
            func.instruction(&Instruction::Br(0)); // Continue loop (depth 0)
            
            // End loop
            func.instruction(&Instruction::End);
            // End block
            func.instruction(&Instruction::End);
            
            // While always returns void object (represented as 0)
            func.instruction(&Instruction::I32Const(0));
        }

        LirInstr::BlockSeq { instrs } => {
            // Execute sequence of instructions, last value remains on stack
            for instr in instrs {
                emit_instruction(func, ctx, instr)?;
            }
        }

        LirInstr::Call(name) => {
            if let Some(func_idx) = ctx.get_function(name) {
                func.instruction(&Instruction::Call(func_idx));
            } else {
                return Err(format!("Unknown function: {}", name));
            }
        }

        LirInstr::CallIndirect { type_index, .. } => {
            // type_index represents the number of params, look up the actual type index
            let actual_type_idx = ctx.get_call_type(*type_index).unwrap_or(*type_index);
            func.instruction(&Instruction::CallIndirect {
                type_index: actual_type_idx,
                table_index: 0,
            });
        }

        LirInstr::Return => {
            func.instruction(&Instruction::Return);
        }

        LirInstr::GetVTableAddr(class_name) => {
            // Look up the vtable address for this class
            let addr = ctx.get_vtable_address(class_name).unwrap_or(0);
            func.instruction(&Instruction::I32Const(addr as i32));
        }

        LirInstr::BoxBool => {
            // Convert raw i32 boolean (0 or 1) to boxed Bool object pointer
            // Stack: [i32 value] -> [Bool object ptr]
            // if value != 0 { BOOL_TRUE } else { BOOL_FALSE }
            func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I32)));
            func.instruction(&Instruction::I32Const(ctx.get_bool_true_addr() as i32));
            func.instruction(&Instruction::Else);
            func.instruction(&Instruction::I32Const(ctx.get_bool_false_addr() as i32));
            func.instruction(&Instruction::End);
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
            string_equals: 13,
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
