//! IR to WASM instruction emission
//!
//! This module converts from the intermediate representation (LIR)
//! to WASM instructions using wasm-encoder.

use crate::wasm::WasmModule;
use crate::runtime::add_runtime;
use ir::lir::*;
use wasm_encoder::{Function, Instruction, ValType};
use std::collections::HashMap;

/// Emit context for tracking function-local state during code generation
pub struct EmitContext {
    /// Function name to index mapping
    function_map: HashMap<String, u32>,
    /// Label to depth mapping (for br/br_if instructions)
    label_depths: HashMap<String, u32>,
    /// Current label depth
    current_depth: u32,
}

impl EmitContext {
    /// Create a new emission context
    pub fn new() -> Self {
        Self {
            function_map: HashMap::new(),
            label_depths: HashMap::new(),
            current_depth: 0,
        }
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
        Self::new()
    }
}

/// Emit a complete WASM module from LIR program
pub fn emit_module(program: &LirProgram) -> Result<Vec<u8>, String> {
    let mut module = WasmModule::new();
    let mut ctx = EmitContext::new();

    // Initialize memory (1 page = 64KB for static data + heap)
    module.init_memory(16); // 16 pages = 1MB

    // Export memory for debugging
    module.export_memory("memory");

    // Add runtime functions (Object, IO, String)
    let _runtime = add_runtime(&mut module);

    // Register all program functions first (for call references)
    for (i, func) in program.functions.iter().enumerate() {
        // Runtime functions take first indices, so offset by runtime count
        let func_idx = (i + 11) as u32; // 11 runtime functions (2 WASI + 9 COOL)
        ctx.register_function(func.name.clone(), func_idx);
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

    // Emit function bodies
    for func in &program.functions {
        let wasm_func = emit_function(&ctx, func)?;
        module.add_code(wasm_func);
    }

    // Export Main_main as _start (WASI entry point)
    if let Some(main_idx) = ctx.get_function("Main_main") {
        module.export_function("_start", main_idx);
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
            // TODO: Implement allocator call
            // For now, just push a dummy pointer
            func.instruction(&Instruction::I32Const(0x10000));
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
        let mut ctx = EmitContext::new();
        ctx.register_function("test_func".to_string(), 42);
        assert_eq!(ctx.get_function("test_func"), Some(42));
    }

    #[test]
    fn test_lir_type_conversion() {
        assert_eq!(lir_type_to_wasm(&LirType::I32), ValType::I32);
        assert_eq!(lir_type_to_wasm(&LirType::I64), ValType::I64);
    }
}
