# WebAssembly Backend Architecture Design

**Author:** Ritchie (Systems Developer)  
**Date:** 2026-02-27  
**Status:** Proposal  
**Target:** WebAssembly (WASM) with WASI

---

## 1. Executive Summary

This document outlines the architecture for a WebAssembly backend for the COOL language compiler. The design features a three-tier IR (HIR → MIR → LIR), linear memory management with arena allocation, dynamic dispatch via `call_indirect`, and WASI-based I/O.

**Key Decision: WASI vs Custom Imports**  
We will use **WASI** for I/O operations (stdin/stdout) rather than custom imports. This provides:
- Portability across WASI-compliant runtimes (Wasmtime, Wasmer, Node.js)
- Standard file descriptor model (`fd_read`, `fd_write`)
- No need for host-specific shims
- Future compatibility with WASI Preview 2 and Component Model for advanced string handling

---

## 2. Three-Tier Intermediate Representation (IR)

### 2.1 High-Level IR (HIR)
**Purpose:** Typed AST with minimal lowering  
**Characteristics:**
- Direct mapping from parser AST
- Full type information preserved
- Expression-oriented (matches COOL semantics)
- Class/method structure intact

**Example:**
```rust
enum HirExpr {
    Dispatch { object: Box<HirExpr>, method: Symbol, args: Vec<HirExpr>, return_type: TypeId },
    Block { exprs: Vec<HirExpr>, result_type: TypeId },
    Let { name: Symbol, init: Box<HirExpr>, body: Box<HirExpr>, type_: TypeId },
    If { cond: Box<HirExpr>, then_: Box<HirExpr>, else_: Box<HirExpr> },
    // ... arithmetic, comparison, literals
}
```

### 2.2 Mid-Level IR (MIR)
**Purpose:** Stack-based control-flow graph (CFG)  
**Characteristics:**
- Basic blocks with explicit control flow
- Stack-based computation (prepares for WASM)
- Lowered expression forms (if → br_if, let → local.set)
- Type information still present

**Example:**
```rust
struct MirFunction {
    locals: Vec<(Symbol, TypeId)>,
    blocks: Vec<BasicBlock>,
}

struct BasicBlock {
    label: BlockId,
    instructions: Vec<MirInstr>,
    terminator: Terminator,
}

enum MirInstr {
    Push(Value),
    LocalGet(LocalId),
    LocalSet(LocalId),
    Call(FunctionId, Vec<TypeId>),
    CallIndirect { table_index: u32, type_index: u32 },
    MemoryLoad { offset: u32, align: u32 },
    MemoryStore { offset: u32, align: u32 },
    // ... arithmetic, comparison
}

enum Terminator {
    Branch(BlockId),
    BranchIf { target: BlockId, fallthrough: BlockId },
    Return,
}
```

### 2.3 Low-Level IR (LIR)
**Purpose:** Linear WASM-like instruction sequence  
**Characteristics:**
- Flattened basic blocks
- Direct 1:1 mapping to WASM instructions
- All control flow encoded as WASM block/loop/br constructs
- Ready for binary encoding

**Example:**
```rust
enum LirInstr {
    I32Const(i32),
    I32Add,
    LocalGet(u32),
    LocalSet(u32),
    Call(u32),
    CallIndirect { type_idx: u32, table_idx: u32 },
    Block { label: u32 },
    Loop { label: u32 },
    Br(u32),
    BrIf(u32),
    Return,
}
```

---

## 3. Memory Model

### 3.1 Linear Memory Layout
WebAssembly provides a single linear memory space (starting at 0, growable).

**Memory Map:**
```
[ Static Data | Heap | Stack (grows down) ]
0x0000         0x10000 (64KB boundary)
```

- **Static Data (0x0 - 0xFFFF):** String literals, class metadata, vtables (read-only at runtime)
- **Heap (0x10000+):** Dynamic objects allocated via arena allocator
- **Stack:** Not explicitly managed (WASM provides implicit operand stack for locals/temps)

### 3.2 Arena Allocator
**Why Arena?** COOL has no explicit `free()` — objects live for the program's duration. Arena allocation is simple and efficient.

**Allocation Strategy:**
```rust
struct Arena {
    memory_base: u32,  // Current heap pointer (starts at 0x10000)
    memory_size: u32,  // Current memory size in pages (64KB each)
}

fn allocate(size: u32) -> u32 {
    let ptr = memory_base;
    memory_base += size;
    if memory_base > memory_size * 65536 {
        grow_memory();  // WASM memory.grow instruction
    }
    ptr
}
```

**No Garbage Collection (Initial Implementation):**  
Objects are never freed. For production, we could add:
- Reference counting
- Mark-and-sweep GC (traversing object graph)
- WASM GC proposal integration (future)

---

## 4. Object Layout and Dynamic Dispatch

### 4.1 Object Layout
Each COOL object is a contiguous memory block:

```
+------------------+
| class_tag (i32)  |  +0  (unique integer per class)
+------------------+
| size (i32)       |  +4  (total object size in bytes)
+------------------+
| vtable_ptr (i32) |  +8  (pointer to vtable in static memory)
+------------------+
| attr_0 (i32)     |  +12 (first attribute)
+------------------+
| attr_1 (i32)     |  +16 (second attribute)
+------------------+
| ...              |
+------------------+
```

**Field Access:**
```wasm
;; Get attr_1 from object at stack top
local.get $object_ptr
i32.const 16          ;; offset to attr_1
i32.add
i32.load              ;; load 32-bit value
```

### 4.2 VTable Layout
Each class has a vtable stored in static memory:

```
+------------------+
| method_0_ptr     |  Function index for method 0
+------------------+
| method_1_ptr     |  Function index for method 1
+------------------+
| ...              |
+------------------+
```

**VTable Indexing:**
- Method indices are assigned during semantic analysis (Hopper's job)
- Child classes inherit parent methods → same index for overrides
- New methods get new indices appended

### 4.3 Dynamic Dispatch via `call_indirect`
WASM's `call_indirect` instruction enables dynamic dispatch:

```wasm
;; obj.method(arg1, arg2)
local.get $arg1
local.get $arg2
local.get $obj_ptr

;; Load vtable pointer
local.get $obj_ptr
i32.const 8           ;; vtable_ptr offset
i32.add
i32.load              ;; vtable address

;; Load method pointer from vtable
i32.const 4           ;; method index * 4 (method_1)
i32.add
i32.load              ;; function index

;; Call via function table
call_indirect (type $method_sig)
```

**Function Table:**
All functions (including methods) are stored in WASM's table:
```wasm
(table $vtable 100 funcref)  ;; Max 100 functions
(elem $vtable (offset (i32.const 0)) $Main.main $IO.out_string ...)
```

---

## 5. Runtime Support and I/O

### 5.1 WASI I/O Model
We use **WASI** (not custom imports) for standard I/O:

**WASI Functions:**
```wasm
(import "wasi_snapshot_preview1" "fd_write"
  (func $fd_write (param i32 i32 i32 i32) (result i32)))
(import "wasi_snapshot_preview1" "fd_read"
  (func $fd_read (param i32 i32 i32 i32) (result i32)))
```

**IO Class Methods:**
- `out_string(s: String)` → WASI `fd_write` to stdout (fd=1)
- `out_int(i: Int)` → Convert int to string, then `fd_write`
- `in_string()` → WASI `fd_read` from stdin (fd=0)
- `in_int()` → `fd_read` then parse string to int

**String Representation:**
Strings are objects with:
```
+------------------+
| class_tag        |  (String class)
+------------------+
| size             |
+------------------+
| vtable_ptr       |
+------------------+
| length (i32)     |  +12 (string length)
+------------------+
| data_ptr (i32)   |  +16 (pointer to UTF-8 bytes)
+------------------+
```

String data stored separately in linear memory (or inline for small strings).

### 5.2 WASI vs Custom Imports — Decision Rationale

**Why WASI?**
1. **Portability:** Run on Wasmtime, Wasmer, Node.js without custom host code
2. **Standard Interface:** `fd_read`/`fd_write` are POSIX-like, well-documented
3. **Future-Proof:** WASI Preview 2 and Component Model will add native string types
4. **Capability-Based Security:** Host controls access (stdin/stdout must be granted)

**Why Not Custom Imports?**
1. **Host-Specific:** Each runtime requires custom shim implementation
2. **String Marshalling:** Manual pointer/length passing is error-prone
3. **Fragmentation:** No interop with other WASM tools/libraries

**Trade-Off:**  
WASI adds slight overhead (file descriptor model for simple I/O), but the portability and standards compliance outweigh this for a teaching compiler.

---

## 6. Compilation Pipeline

### 6.1 AST → HIR
- Walk parser AST
- Resolve types (from semantic analysis)
- Create HIR expressions with type annotations

### 6.2 HIR → MIR
- Convert expressions to basic blocks
- Lower control flow (if → br_if, while → loop + br_if)
- Flatten let bindings to local.set
- Generate vtables and object layouts
- Build function table

### 6.3 MIR → LIR
- Flatten basic blocks into linear instruction sequence
- Resolve block labels
- Encode control flow as WASM structured control (block/loop/br)

### 6.4 LIR → WASM Binary
- Encode LIR instructions to WASM bytecode
- Generate WASM module sections:
  - **Type:** Function signatures
  - **Import:** WASI functions
  - **Function:** Function type indices
  - **Table:** Function table for call_indirect
  - **Memory:** Linear memory (initial size)
  - **Global:** Global variables (if needed)
  - **Export:** Entry point (_start for WASI)
  - **Code:** Function bodies

---

## 7. Recommended Rust Crates

### 7.1 WASM Encoding: `wasm-encoder`
**Why:** Low-level, precise control over WASM binary generation

```rust
use wasm_encoder::*;

let mut module = Module::new();

// Type section
let mut types = TypeSection::new();
types.function([ValType::I32], [ValType::I32]);
module.section(&types);

// Code section
let mut functions = CodeSection::new();
let mut func = Function::new([]);
func.instruction(&Instruction::LocalGet(0));
func.instruction(&Instruction::I32Const(42));
func.instruction(&Instruction::I32Add);
func.instruction(&Instruction::End);
functions.function(&func);
module.section(&functions);

let wasm_bytes = module.finish();
```

**Pros:**
- Direct control over every section
- Lightweight (no parsing/validation overhead)
- Official wasmtime project crate

### 7.2 Alternative: `walrus`
**Why:** Higher-level AST-like API for WASM manipulation

```rust
use walrus::*;

let mut module = Module::default();
let mut builder = FunctionBuilder::new(&mut module.types, &[], &[ValType::I32]);
builder.func_body()
    .i32_const(42)
    .i32_const(1)
    .binop(BinaryOp::I32Add);
let func_id = builder.finish(vec![], &mut module.funcs);
module.exports.add("main", func_id);

let wasm_bytes = module.emit_wasm();
```

**Pros:**
- Higher-level abstractions (less boilerplate)
- Automatic validation
- Easier for rapid prototyping

**Cons:**
- More opinionated API
- Slightly heavier dependency

**Recommendation:** Start with `wasm-encoder` for learning and control. Switch to `walrus` if API complexity becomes a bottleneck.

---

## 8. Testing Strategy

### 8.1 Unit Tests
- Test each IR transformation (AST→HIR, HIR→MIR, MIR→LIR)
- Test object layout calculations
- Test vtable generation

### 8.2 Integration Tests
- Compile sample COOL programs to WASM
- Run with Wasmtime: `wasmtime run --invoke _start program.wasm`
- Compare output against expected results

### 8.3 WASI Compliance
- Test I/O functions with stdin/stdout redirection
- Verify portability across Wasmtime, Wasmer, Node.js WASI

---

## 9. Open Questions and Future Work

### 9.1 Garbage Collection
**Current:** Arena allocation (no collection)  
**Future Options:**
- Reference counting (simple, but cycles leak)
- Mark-and-sweep GC (requires root set tracking)
- WASM GC proposal (native GC types: `struct`, `array`, `ref`)

### 9.2 String Optimization
**Current:** Pointer-based strings in linear memory  
**Future:** WASI Preview 2 + Component Model supports native string passing across boundaries (no manual marshalling)

### 9.3 Performance Optimizations
- Inline small methods (eliminate call overhead)
- Constant folding in MIR
- Dead code elimination
- Tail call optimization (WASM tail call proposal)

### 9.4 Exception Handling
COOL has runtime errors (division by zero, dispatch on void). Options:
- Trap (WASM `unreachable` — abrupt termination)
- Return error codes (requires wrapping all operations)
- WASM exception handling proposal (try/catch blocks)

---

## 10. Implementation Phases

### Phase 1: Core Infrastructure (Week 1)
- Define HIR/MIR/LIR types
- Implement AST → HIR transformation
- Basic WASM module generation (empty functions)

### Phase 2: Memory and Objects (Week 2)
- Arena allocator
- Object layout and field access
- String representation

### Phase 3: Dynamic Dispatch (Week 3)
- VTable generation
- `call_indirect` implementation
- Method dispatch logic

### Phase 4: I/O and Runtime (Week 4)
- WASI integration (fd_read/fd_write)
- IO class methods
- String/int conversion

### Phase 5: Control Flow and Expressions (Week 5)
- If/while lowering
- Arithmetic/comparison operators
- Let bindings and blocks

### Phase 6: Testing and Validation (Week 6)
- End-to-end tests with sample programs
- WASI runtime testing (Wasmtime, Wasmer, Node.js)
- Performance benchmarking

---

## 11. References and Resources

### WASM Specifications
- [WebAssembly Core Spec](https://webassembly.github.io/spec/core/)
- [WASM Binary Format](https://webassembly.github.io/spec/core/binary/index.html)

### WASI
- [WASI GitHub](https://github.com/WebAssembly/WASI)
- [WASI.dev Documentation](https://wasi.dev/)
- [WASI I/O Proposal](https://github.com/WebAssembly/wasi-io)

### Rust Crates
- [wasm-encoder](https://docs.rs/wasm-encoder/) — Low-level WASM binary encoding
- [walrus](https://docs.rs/walrus/) — High-level WASM AST manipulation
- [wasmparser](https://docs.rs/wasmparser/) — WASM binary parsing (for validation)

### WASM Runtimes
- [Wasmtime](https://wasmtime.dev/) — Fast, secure WASM runtime (Rust)
- [Wasmer](https://wasmer.io/) — Portable WASM runtime
- [Node.js WASI](https://nodejs.org/api/wasi.html) — WASI support in Node.js

---

## 12. Summary

This design provides a solid foundation for a WebAssembly backend for COOL:
- **Three-tier IR** balances abstraction and control
- **Linear memory + arena allocation** matches COOL's memory model
- **call_indirect + vtables** enables dynamic dispatch
- **WASI I/O** ensures portability without custom host code
- **wasm-encoder** gives precise control over binary generation

The architecture is modular, testable, and ready for implementation.

---

**Next Steps:**
1. Get Hopper's feedback on IR design (especially HIR → MIR boundary)
2. Create `codegen/` crate with IR type definitions
3. Prototype simple WASM module generation (Hello World)
4. Implement arena allocator and object layout

**Questions for Team:**
- Should we support WASM GC proposal from the start, or defer?
- Do we need a separate `runtime/` crate for WASI wrappers, or inline in codegen?
- What's the performance target? (Naive codegen first, or optimize early?)
