//! WebAssembly module builder
//!
//! Provides a high-level API for constructing WASM modules with proper
//! section management, type tracking, and export handling.

use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ElementSection, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, MemorySection, MemoryType, Module,
    StartSection, TableSection, TableType, TypeSection, ValType,
};

/// WASM module builder for COOL programs
///
/// Manages the construction of a WebAssembly module including:
/// - Memory layout (static data + heap)
/// - Type signatures for functions
/// - Function table for dynamic dispatch
/// - WASI imports for I/O
/// - Export of entry point (_start)
pub struct WasmModule {
    module: Module,
    types: TypeSection,
    imports: ImportSection,
    functions: FunctionSection,
    tables: TableSection,
    memory: MemorySection,
    globals: GlobalSection,
    exports: ExportSection,
    start: Option<StartSection>,
    elements: ElementSection,
    code: CodeSection,
    data: DataSection,
    next_type_idx: u32,
    next_func_idx: u32,
    /// Type cache for deduplication: (params, results) -> type_idx
    type_cache: std::collections::HashMap<(Vec<ValType>, Vec<ValType>), u32>,
}

impl WasmModule {
    /// Create a new empty WASM module
    pub fn new() -> Self {
        Self {
            module: Module::new(),
            types: TypeSection::new(),
            imports: ImportSection::new(),
            functions: FunctionSection::new(),
            tables: TableSection::new(),
            memory: MemorySection::new(),
            globals: GlobalSection::new(),
            exports: ExportSection::new(),
            start: None,
            elements: ElementSection::new(),
            code: CodeSection::new(),
            data: DataSection::new(),
            next_type_idx: 0,
            next_func_idx: 0,
            type_cache: std::collections::HashMap::new(),
        }
    }

    /// Initialize linear memory
    ///
    /// Sets up a single linear memory with:
    /// - Initial size: 1 page (64KB) for static data
    /// - Maximum size: None (growable)
    pub fn init_memory(&mut self, initial_pages: u32) {
        self.memory.memory(MemoryType {
            minimum: initial_pages.into(),
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
    }

    /// Add a function type signature
    ///
    /// Returns the type index for use in function declarations.
    /// Uses caching to deduplicate identical type signatures.
    pub fn add_type(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let key = (params.clone(), results.clone());
        if let Some(&idx) = self.type_cache.get(&key) {
            return idx;
        }

        self.types.ty().function(params, results);
        let idx = self.next_type_idx;
        self.next_type_idx += 1;
        self.type_cache.insert(key, idx);
        idx
    }

    /// Import a WASI function
    ///
    /// Imports a function from the "wasi_snapshot_preview1" module.
    pub fn import_wasi(&mut self, name: &str, type_idx: u32) {
        self.imports.import(
            "wasi_snapshot_preview1",
            name,
            wasm_encoder::EntityType::Function(type_idx),
        );
        self.next_func_idx += 1;
    }

    /// Add a function definition
    ///
    /// Associates a function with its type signature.
    pub fn add_function(&mut self, type_idx: u32) -> u32 {
        self.functions.function(type_idx);
        let idx = self.next_func_idx;
        self.next_func_idx += 1;
        idx
    }

    /// Add function code
    ///
    /// Provides the implementation for a function.
    pub fn add_code(&mut self, func: Function) {
        self.code.function(&func);
    }

    /// Export a function
    ///
    /// Makes a function visible to the host environment.
    pub fn export_function(&mut self, name: &str, func_idx: u32) {
        self.exports.export(name, ExportKind::Func, func_idx);
    }

    /// Export memory
    ///
    /// Makes linear memory accessible to the host.
    pub fn export_memory(&mut self, name: &str) {
        self.exports.export(name, ExportKind::Memory, 0);
    }

    /// Initialize function table for dynamic dispatch
    ///
    /// Creates a table to hold function references for call_indirect.
    pub fn init_table(&mut self, initial_size: u32, max_size: Option<u32>) {
        self.tables.table(TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            minimum: initial_size as u64,
            maximum: max_size.map(|s| s as u64),
            table64: false,
            shared: false,
        });
    }

    /// Add an element section to initialize the function table
    ///
    /// Populates the function table with function references for call_indirect.
    pub fn add_element_section(&mut self, func_indices: &[u32]) {
        use std::borrow::Cow;
        use wasm_encoder::Elements;
        self.elements.active(
            Some(0),                                // table index
            &wasm_encoder::ConstExpr::i32_const(0), // offset
            Elements::Functions(Cow::Borrowed(func_indices)),
        );
    }

    /// Add static data to linear memory
    ///
    /// Places data at a specific offset (typically for string literals, vtables).
    pub fn add_data(&mut self, offset: u32, data: Vec<u8>) {
        self.data
            .active(0, &wasm_encoder::ConstExpr::i32_const(offset as i32), data);
    }

    /// Add a global variable
    ///
    /// Creates a mutable or immutable global with an initial value.
    pub fn add_global(&mut self, val_type: ValType, mutable: bool, init_value: i32) {
        self.globals.global(
            GlobalType {
                val_type,
                mutable,
                shared: false,
            },
            &ConstExpr::i32_const(init_value),
        );
    }

    /// Set the start function
    ///
    /// Specifies which function to execute when the module is instantiated.
    pub fn set_start(&mut self, func_idx: u32) {
        self.start = Some(StartSection {
            function_index: func_idx,
        });
    }

    /// Finalize and encode the WASM module
    ///
    /// Assembles all sections into a complete WASM binary.
    pub fn finish(mut self) -> Vec<u8> {
        // Add sections in the correct order (per WASM spec)
        if self.types.len() > 0 {
            self.module.section(&self.types);
        }
        if self.imports.len() > 0 {
            self.module.section(&self.imports);
        }
        if self.functions.len() > 0 {
            self.module.section(&self.functions);
        }
        if self.tables.len() > 0 {
            self.module.section(&self.tables);
        }
        if self.memory.len() > 0 {
            self.module.section(&self.memory);
        }
        if self.globals.len() > 0 {
            self.module.section(&self.globals);
        }
        if self.exports.len() > 0 {
            self.module.section(&self.exports);
        }
        if let Some(ref start) = self.start {
            self.module.section(start);
        }
        if self.elements.len() > 0 {
            self.module.section(&self.elements);
        }
        if self.code.len() > 0 {
            self.module.section(&self.code);
        }
        if self.data.len() > 0 {
            self.module.section(&self.data);
        }

        self.module.finish()
    }
}

impl Default for WasmModule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::Instruction;

    #[test]
    fn test_empty_module() {
        let module = WasmModule::new();
        let bytes = module.finish();
        // WASM magic number + version
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]); // "\0asm"
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]); // version 1
    }

    #[test]
    fn test_simple_function() {
        let mut module = WasmModule::new();

        // (func (result i32) i32.const 42)
        let type_idx = module.add_type(vec![], vec![ValType::I32]);
        let func_idx = module.add_function(type_idx);

        let mut func = Function::new([]);
        func.instruction(&Instruction::I32Const(42));
        func.instruction(&Instruction::End);
        module.add_code(func);

        module.export_function("answer", func_idx);

        let bytes = module.finish();
        assert!(bytes.len() > 8); // More than just header
    }
}
