# WebAssembly Backend Architecture

**Date:** 2026-02-27  
**By:** Ritchie (Systems Developer)  
**Status:** Proposed  

---

## Decision: Target WebAssembly Instead of x86/ARM

**What:** Change compiler backend target from x86-64/ARM64 to WebAssembly (WASM)

**Why:**
- **Portability:** WASM runs on any platform without recompilation (browser, server, edge, embedded)
- **Simplicity:** Stack-based instruction set is simpler than register-based (x86/ARM)
- **Modern Tooling:** Excellent Rust ecosystem (`wasm-encoder`, `walrus`, Wasmtime runtime)
- **Learning Value:** WASM is increasingly important for web, serverless, and cloud-native applications
- **Safety:** Built-in sandboxing and memory safety (no buffer overflows, no code injection)

**Impact:**
- Ritchie's charter updated: expertise now in WASM, not x86/ARM
- All backend design work targets WASM binary format
- Testing with WASI-compliant runtimes (Wasmtime, Wasmer, Node.js)

---

## Decision: Use WASI for I/O (Not Custom Imports)

**What:** Use WASI (WebAssembly System Interface) for all I/O operations instead of custom host imports

**Why:**
1. **Portability:** WASI is supported by all major runtimes (Wasmtime, Wasmer, Node.js) — no custom host code required
2. **Standard Interface:** `fd_read`/`fd_write` provide POSIX-like file descriptor model for stdin/stdout
3. **Future-Proof:** WASI Preview 2 and Component Model will add native string types, eliminating manual marshalling
4. **Capability-Based Security:** Host controls access to stdin/stdout explicitly (safer than global access)
5. **Interoperability:** WASM modules using WASI can work with other WASI tools/libraries

**Alternative Considered:** Custom imports (e.g., `env.print_string`, `env.read_int`)
- **Pros:** Simpler for toy compiler, direct control
- **Cons:** Host-specific (every runtime needs custom shims), manual string marshalling (pointer+length), no interop, not standards-compliant

**Trade-Off:**
- WASI adds slight overhead (file descriptor abstraction for simple I/O)
- But portability and standards compliance far outweigh this for a teaching/production compiler

**Implementation:**
- IO class methods map to WASI:
  - `out_string(s: String)` → `fd_write(1, ...)`  (stdout)
  - `out_int(i: Int)` → int-to-string + `fd_write(1, ...)`
  - `in_string()` → `fd_read(0, ...)`  (stdin)
  - `in_int()` → `fd_read(0, ...)` + string-to-int

---

## Decision: Three-Tier IR (HIR → MIR → LIR)

**What:** Use a three-level intermediate representation for progressive lowering

**Structure:**
1. **HIR (High-Level IR):** Typed AST, expression-oriented, minimal lowering from parser
2. **MIR (Mid-Level IR):** Stack-based CFG with basic blocks, control flow explicit, prepares for WASM
3. **LIR (Low-Level IR):** Linear WASM-like instructions, 1:1 mapping to WASM bytecode

**Why:**
- **Separation of Concerns:** Each tier handles one aspect (types → control flow → encoding)
- **Testability:** Can test each transformation independently
- **Optimization Opportunities:** MIR is ideal for constant folding, dead code elimination
- **Clarity:** Each tier has a clear purpose and well-defined output

**Alternative Considered:** Direct AST → WASM
- **Pros:** Simpler, fewer transformations
- **Cons:** Hard to optimize, testing difficult, control flow and type lowering mixed together

**Trade-Off:**
- More code and complexity upfront
- But much easier to maintain, test, and optimize long-term

---

## Decision: Arena Allocator (No GC Initially)

**What:** Use arena allocation for COOL objects — no garbage collection in initial implementation

**Why:**
- **COOL Semantics:** No explicit `free()` in language — objects live for program duration
- **Simplicity:** Arena is trivial to implement (`memory_base += size`, plus `memory.grow`)
- **Performance:** Zero overhead (no GC pauses, no reference counting)
- **Sufficient for Samples:** All test programs are short-lived (no memory exhaustion)

**Future Work:**
- Reference counting (simple, but cycles leak)
- Mark-and-sweep GC (requires root set tracking)
- WASM GC proposal integration (native GC types: `struct`, `array`, `ref`)

**Trade-Off:**
- Long-running programs will exhaust memory
- But acceptable for a teaching compiler and test programs

---

## Decision: Dynamic Dispatch via `call_indirect`

**What:** Use WASM's `call_indirect` instruction with vtables for method dispatch

**Why:**
- **Native WASM Feature:** `call_indirect` is designed for dynamic dispatch (type-safe)
- **Efficient:** Single indirect call through function table (no hash lookup)
- **Standard Pattern:** Matches C++ virtual functions, Rust trait objects

**Implementation:**
- Each class has a vtable in static memory (array of function indices)
- Objects store vtable pointer at fixed offset (8 bytes)
- Method calls: load vtable → load function index → `call_indirect`

**Alternative Considered:** Hash-based dispatch (method name → function)
- **Pros:** More flexible (dynamic method resolution)
- **Cons:** Slower (hash lookup), more complex runtime

**Trade-Off:**
- VTable approach is faster and simpler
- Requires method index assignment during semantic analysis (Hopper's job)

---

## Decision: Object Layout (Header + Attributes)

**What:** Objects are contiguous memory blocks with fixed header

**Layout:**
```
+------------------+
| class_tag (i32)  |  +0  (unique integer per class)
| size (i32)       |  +4  (total object size in bytes)
| vtable_ptr (i32) |  +8  (pointer to vtable in static memory)
| attr_0 (i32)     |  +12 (first attribute)
| attr_1 (i32)     |  +16 (second attribute)
| ...              |
+------------------+
```

**Why:**
- **Type Checking:** `class_tag` enables runtime type checks (`case` expressions)
- **GC Support:** `size` enables memory traversal (future mark-and-sweep)
- **Dispatch:** `vtable_ptr` enables dynamic method calls
- **Simplicity:** Fixed offsets for all fields (no indirection)

**Trade-Off:**
- 12-byte header overhead per object
- But necessary for COOL semantics (type checking, inheritance, dispatch)

---

## Decision: Use `wasm-encoder` for Binary Generation

**What:** Use the `wasm-encoder` Rust crate for WASM binary encoding

**Why:**
- **Low-Level Control:** Precise control over every WASM section and instruction
- **Lightweight:** No parsing/validation overhead (we're generating, not transforming)
- **Official:** Part of the Wasmtime project (well-maintained, standards-compliant)
- **Direct Mapping:** LIR instructions map 1:1 to `wasm-encoder` API

**Alternative Considered:** `walrus` (higher-level AST-like API)
- **Pros:** Less boilerplate, automatic validation, easier for rapid prototyping
- **Cons:** More opinionated, slightly heavier dependency

**Trade-Off:**
- `wasm-encoder` requires more manual work (section ordering, index management)
- But gives us full understanding and control of WASM binary format
- Can switch to `walrus` later if API complexity becomes a bottleneck

---

## Implementation Timeline

**Phase 1 (Week 1):** Core IR infrastructure (HIR/MIR/LIR types, AST→HIR)  
**Phase 2 (Week 2):** Memory and objects (arena allocator, layout, field access)  
**Phase 3 (Week 3):** Dynamic dispatch (vtables, call_indirect)  
**Phase 4 (Week 4):** I/O and runtime (WASI integration, IO class)  
**Phase 5 (Week 5):** Control flow and expressions (if/while, arithmetic, let)  
**Phase 6 (Week 6):** Testing and validation (samples, WASI runtimes)  

---

## Open Questions

1. **GC Strategy:** Should we implement GC early, or defer until Phase 2 of project?
2. **Runtime Crate:** Should WASI wrappers live in a separate `runtime/` crate, or inline in `codegen/`?
3. **Performance Targets:** Naive codegen first (fast compile), or optimize early (fast runtime)?
4. **Hopper Coordination:** HIR → MIR boundary — who owns type information? (Ritchie needs types for dispatch)

---

## References

- Design Document: `.squad/agents/ritchie/wasm-backend-design.md`
- Updated Charter: `.squad/agents/ritchie/charter.md`
- History/Learnings: `.squad/agents/ritchie/history.md`

---

**Next Actions:**
1. Get team feedback on these decisions (especially Hopper for IR interface)
2. Create `codegen/` crate with IR types
3. Prototype simple WASM module (Hello World)
4. Set up WASI testing infrastructure
