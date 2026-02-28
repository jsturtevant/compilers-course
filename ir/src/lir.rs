/// Low-level IR (LIR) - Linear WASM-like instructions ready for codegen

/// LIR Program
#[derive(Debug, Clone)]
pub struct LirProgram {
    pub functions: Vec<LirFunction>,
    pub globals: Vec<LirGlobal>,
    pub vtables: Vec<VTable>,
    pub string_data: Vec<StringData>,
}

/// String literal data for data section
#[derive(Debug, Clone)]
pub struct StringData {
    pub offset: u32,
    pub value: String,
}

/// LIR Function
#[derive(Debug, Clone)]
pub struct LirFunction {
    pub name: String,
    pub params: Vec<LirType>,
    pub return_type: LirType,
    pub locals: Vec<LirLocal>,
    pub body: Vec<LirInstr>,
}

/// LIR Local variable
#[derive(Debug, Clone)]
pub struct LirLocal {
    pub index: u32,
    pub typ: LirType,
}

/// LIR Global variable
#[derive(Debug, Clone)]
pub struct LirGlobal {
    pub name: String,
    pub typ: LirType,
    pub init: i32,
}

/// Virtual table for dynamic dispatch
#[derive(Debug, Clone)]
pub struct VTable {
    pub class_name: String,
    pub methods: Vec<String>, // Method function names in order
}

/// LIR Type (WASM types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LirType {
    I32, // 32-bit integer (objects are pointers)
    I64, // 64-bit integer (future use)
    Void, // No return value
}

/// LIR Label for control flow
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(pub String);

/// LIR Instruction
#[derive(Debug, Clone)]
pub enum LirInstr {
    // Stack operations
    /// Push constant onto stack
    I32Const(i32),
    
    /// Duplicate top of stack
    Dup,

    /// Pop top of stack (discard)
    Drop,

    // Arithmetic operations
    /// Add two i32 values
    I32Add,

    /// Subtract two i32 values
    I32Sub,

    /// Multiply two i32 values
    I32Mul,

    /// Divide two i32 values (signed)
    I32DivS,

    /// Negate i32 value
    I32Neg,

    // Comparison operations
    /// Less than (signed)
    I32LtS,

    /// Less than or equal (signed)
    I32LeS,

    /// Equal
    I32Eq,

    /// Not equal
    I32Ne,

    // Boolean operations
    /// Equal to zero (logical not)
    I32Eqz,

    // Local variables
    /// Get local variable
    LocalGet(u32),

    /// Set local variable
    LocalSet(u32),

    /// Get and increment local (for temp usage)
    LocalTee(u32),

    // Global variables
    /// Get global variable
    GlobalGet(u32),

    /// Set global variable
    GlobalSet(u32),

    // Memory operations
    /// Load i32 from memory at address (with offset and alignment)
    I32Load { offset: u32, align: u32 },

    /// Store i32 to memory at address (with offset and alignment)
    I32Store { offset: u32, align: u32 },

    /// Allocate memory (bump allocator)
    /// Stack: [size] -> [ptr]
    Alloc,

    // Control flow
    /// Label for jump target
    Label(Label),

    /// Unconditional jump to label
    Jump(Label),

    /// Conditional jump if top of stack is non-zero
    JumpIf(Label),

    /// Conditional jump if top of stack is zero
    JumpIfNot(Label),

    /// Block start (WASM block)
    Block { label: Label },

    /// Block end
    End,

    /// Loop start (WASM loop)
    Loop { label: Label },

    /// Branch to label (WASM br)
    Br(u32),

    /// Branch if non-zero (WASM br_if)
    BrIf(u32),

    // Function calls
    /// Direct function call
    Call(String),

    /// Indirect function call via vtable
    /// Stack: [object_ptr, arg1, ..., argN] -> [result]
    CallIndirect { 
        type_index: u32, 
        method_index: u32 
    },

    /// Return from function
    Return,

    // Special operations
    /// Trap (runtime error)
    Unreachable,

    /// No operation
    Nop,

    // Comments for debugging
    Comment(String),
}

impl LirInstr {
    /// Create a comment instruction for debugging
    pub fn comment(msg: impl Into<String>) -> Self {
        LirInstr::Comment(msg.into())
    }
}

impl Label {
    pub fn new(name: impl Into<String>) -> Self {
        Label(name.into())
    }

    pub fn fresh(prefix: &str, id: usize) -> Self {
        Label(format!("{}_{}", prefix, id))
    }
}
