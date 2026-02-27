# WASM Architecture Decisions

**Author:** 🏗️ Turing (Lead Architect)  
**Date:** 2026-02-27  
**Status:** Proposed (awaiting team review)

---

## Decision 1: Target WebAssembly Instead of x86/ARM

**Decision:** The COOL compiler will target **WebAssembly (WASM)** as its sole backend.

**Rationale:**
- Eliminates dual-backend complexity (no x86 + ARM + calling conventions)
- WASM's stack-based model matches COOL's expression semantics
- Runs everywhere: browsers (via JS), CLI (via wasmtime/wasmer), embedded
- No register allocation required — WASM locals handle this
- Structured control flow (if/block/loop) maps directly from COOL

**Trade-offs:**
- ❌ Lose native performance (but acceptable for educational compiler)
- ❌ Can't debug with gdb/lldb directly (but WASM has its own debuggers)
- ✅ Much simpler codegen (single target, no ABI variations)
- ✅ More accessible output (anyone can run in browser)

**Alternatives Rejected:**
- x86-64 only: Still complex (System V ABI, register allocation)
- LLVM backend: Too heavy for educational project

---

## Decision 2: Three-Tier IR (HIR → MIR → LIR)

**Decision:** Use three intermediate representations:
1. **HIR (High-level IR):** Typed AST with resolved names
2. **MIR (Mid-level IR):** Stack-based CFG with explicit control flow
3. **LIR (Low-level IR):** Linear instructions mapping 1:1 to WASM

**Rationale:**
- Each IR has a single responsibility (type checking, control flow, instruction selection)
- Easier to test each lowering pass independently
- MIR's stack-based nature matches WASM naturally

**Alternative Rejected:**
- Direct AST→WASM: Too big a jump; hard to debug
- Single IR: Would be too complex or too simple

---

## Decision 3: Stack-Based MIR (Not SSA)

**Decision:** MIR uses a **stack-based** model, not SSA (Static Single Assignment).

**Rationale:**
- WASM is a stack machine; MIR mirrors execution model
- No φ-nodes needed (SSA complexity avoided)
- Expression evaluation order is explicit in stack operations
- Local variables use explicit push/pop, matching WASM locals

**Trade-offs:**
- ❌ Harder to apply classic optimizations (SSA excels here)
- ✅ Simpler implementation for educational compiler
- ✅ Direct lowering to WASM

---

## Decision 4: Dynamic Dispatch via call_indirect + Vtables

**Decision:** Implement COOL's dynamic dispatch using WASM's `call_indirect` with vtables in linear memory.

**Implementation:**
1. Each class has a vtable (array of function indices)
2. Object header contains vtable pointer at offset 8
3. Method dispatch: load vtable ptr → index by slot → `call_indirect`

**Method Slot Assignment:**
- Parent class methods first (inherit slots)
- Override replaces parent's slot
- New methods append to vtable

**Example:**
```
class A { foo() → slot 0, bar() → slot 1 }
class B inherits A { override foo() → slot 0, baz() → slot 2 }
```

---

## Decision 5: Arena Allocator for MVP (No GC)

**Decision:** Use a **bump allocator** (arena) for memory management in MVP.

**Rationale:**
- COOL course programs are small; unlikely to exhaust memory
- Avoids GC complexity (no barriers, no root scanning, no cycle detection)
- Simplifies codegen enormously

**Future Path:**
1. Reference counting (simple, deterministic, cycles leak)
2. Mark-sweep GC (accurate, requires stack walking)
3. WASM GC proposal (best long-term, still experimental)

**Trade-off:**
- ❌ Memory leaks for programs with allocation in loops
- ✅ Dramatically simpler implementation

---

## Decision 6: IO via Host Imports (Dual Runtime)

**Decision:** IO class methods are **WASM imports**, bridged to host environment.

**Import Signatures:**
```wasm
(import "cool" "out_string" (func $out_string (param i32 i32)))
(import "cool" "out_int" (func $out_int (param i32)))
(import "cool" "in_string" (func $in_string (result i32)))
(import "cool" "in_int" (func $in_int (result i32)))
```

**Dual Runtime:**
| Environment | Implementation |
|-------------|----------------|
| Browser | JavaScript glue (~20 lines) |
| CLI | WASI fd_read/fd_write |

**Rationale:**
- Clean separation: compiler produces pure WASM
- Runtime is pluggable and environment-specific
- Enables browser demos without modification

---

## Decision 7: Object Layout in Linear Memory

**Decision:** Objects stored in linear memory with fixed header layout.

**Layout:**
```
Offset  Size  Field
0       4     class_tag (i32) — for case dispatch
4       4     size (i32) — for copy()
8       4     vtable_ptr (i32) — points to method table
12+     4*n   attributes (i32 each, boxed or primitive)
```

**String Layout:**
```
Offset  Size  Field
0       4     length (i32)
4+      n     UTF-8 data (no null terminator)
```

**Rationale:**
- Fixed header simplifies GC (future), copy(), type_name()
- Vtable pointer at known offset enables dispatch
- Length-prefixed strings avoid NUL issues

---

## Action Required

These decisions need team review:

- [ ] **Hopper:** Acknowledge semantic analysis scope (SELF_TYPE, built-ins)
- [ ] **Ritchie:** Acknowledge IR + codegen ownership
- [ ] **Stroustrup:** Review Rust patterns for MIR/LIR data structures
- [ ] **Knuth:** Update architecture documentation

Once acknowledged, move this file to `.squad/decisions/` (out of inbox).

---

**Signed:** 🏗️ Turing, Lead Architect
