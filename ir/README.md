# IR Crate - Intermediate Representation

This crate implements a two-tier IR for the COOL compiler:

## Architecture

```
AST → HIR → LIR → WASM
```

1. **HIR (High-level IR)**: Typed AST with resolved types and method dispatch info
2. **LIR (Low-level IR)**: Linear WASM-like instructions ready for codegen

## Modules

### `ast_to_hir.rs` - AST → HIR Lowering

Transforms the parser's AST into HIR with full type information.

**Main Entry Point:**
```rust
pub fn lower_program(
    program: &Program,
    analyzer: &SemanticAnalyzer,
) -> Result<HirProgram, String>
```

**Features:**
- Resolves all types from semantic analysis
- Builds vtables for method dispatch
- Assigns class tags for runtime type checks
- Converts all expression types to HIR
- Tracks method indices for efficient dispatch

**Supported Expressions:**
- Literals: Int, String, Bool
- Arithmetic: Plus, Minus, Times, Divide
- Comparison: Lt, Le, Eq
- Logical: Not, IsVoid, Negate
- Variables: Id, Assignment
- Dispatch: Dynamic and static method calls
- Control flow: If, While, Block, Let, Case
- Objects: New, FuncCall

### `hir.rs` - High-level IR Types

Defines the HIR data structures:
- `HirProgram` - Program with classes
- `HirClass` - Class with attributes and methods
- `HirMethod` - Method with formals and body
- `HirExpr` - Typed expressions
- `TypeId` - Resolved type identifiers
- `DispatchInfo` - Method dispatch metadata

### `lower.rs` - HIR → LIR Lowering

Transforms HIR into LIR (WASM-like instructions).

**Status:** Partial implementation (stubs for most expressions)

### `lir.rs` - Low-level IR Types

Defines the LIR data structures for WASM codegen.

## Usage Example

```rust
use ir::lower_program;
use parser::ast::Program;
use semant::SemanticAnalyzer;

// Parse and analyze
let program: Program = /* ... */;
let mut analyzer = SemanticAnalyzer::new();
analyzer.analyze(&program)?;

// Lower to HIR
let hir = lower_program(&program, &analyzer)?;

// Access HIR data
for class in &hir.classes {
    println!("Class: {}", class.name);
    for method in &class.methods {
        println!("  Method: {} (vtable index: {})",
                 method.name, method.vtable_index);
    }
}
```

## Testing

Run tests with:
```bash
cargo test -p ir
```

Current test coverage:
- ✓ Simple class lowering
- ✓ Arithmetic expressions
- ✓ Control flow (if-then-else)
- ✓ LIR instruction generation (basic)

## Dependencies

- `parser` - AST definitions
- `semant` - Semantic analysis and type checking

## Next Steps

1. Complete HIR → LIR lowering for all expression types
2. Add more comprehensive tests
3. Implement optimization passes
4. Add debug information to IR nodes
